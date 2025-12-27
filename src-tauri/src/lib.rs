// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use sqlx::{Pool, Sqlite};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::Manager;

mod db;
mod models;
pub mod monitor;
pub mod process_monitor;

use models::AllTimeTotals;
use models::AppMetrics;
use models::ResetDatabaseResponse;
use process_monitor::ProcessAccumulators;
use std::env;
use std::fs;
use sysinfo::{Pid, ProcessesToUpdate, System};

// Database pool state wrapper
pub struct DbPool(pub Arc<Mutex<Option<Pool<Sqlite>>>>);

// Process accumulators state wrapper
pub struct ProcessAccumulatorsState(pub ProcessAccumulators);

// Reset signal wrapper
pub struct ResetSignal(pub Arc<AtomicBool>);

// Shutdown signal wrapper for graceful exit
pub struct ShutdownSignal(pub Arc<AtomicBool>);

// System state wrapper for metrics
pub struct SystemState(pub Mutex<System>);

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_alltime_totals(db_pool: tauri::State<'_, DbPool>) -> Result<AllTimeTotals, String> {
    let pool_opt = {
        let guard = db_pool.0.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };

    if let Some(pool) = pool_opt {
        match db::get_alltime_totals(&pool).await {
            Ok((read_bytes, write_bytes)) => Ok(AllTimeTotals {
                read_bytes,
                write_bytes,
            }),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
async fn save_session_to_alltime(db_pool: tauri::State<'_, DbPool>) -> Result<(), String> {
    let pool_opt = {
        let guard = db_pool.0.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };

    if let Some(pool) = pool_opt {
        // Get the maximum session totals from disk_stats
        match db::get_max_session_totals(&pool).await {
            Ok((read_bytes, write_bytes)) => {
                // Add to alltime totals
                db::update_alltime_totals(&pool, read_bytes, write_bytes)
                    .await
                    .map_err(|e| format!("Database error: {}", e))?;
                Ok(())
            }
            Err(e) => Err(format!("Database error: {}", e)),
        }
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
async fn get_process_history(
    db_pool: tauri::State<'_, DbPool>,
) -> Result<std::collections::HashMap<String, (u64, u64)>, String> {
    let pool_opt = {
        let guard = db_pool.0.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };

    if let Some(pool) = pool_opt {
        db::get_process_history(&pool)
            .await
            .map_err(|e| format!("Database error: {}", e))
    } else {
        Err("Database not initialized".to_string())
    }
}

#[tauri::command]
fn get_database_size(app_handle: tauri::AppHandle) -> Result<ResetDatabaseResponse, String> {
    match db::get_database_size(&app_handle) {
        Ok((size, _)) => Ok(ResetDatabaseResponse {
            db_size_before: size,
            db_size_after: size,
        }),
        Err(e) => Err(format!("Failed to get database size: {}", e)),
    }
}

#[tauri::command]
fn get_app_metrics(
    app_handle: tauri::AppHandle,
    system_state: tauri::State<'_, SystemState>,
) -> Result<AppMetrics, String> {
    let mut sys = system_state.0.lock().map_err(|e| e.to_string())?;

    let pid = Pid::from_u32(std::process::id());
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]));

    let process = sys.process(pid).ok_or("Could not find current process")?;

    let ram_usage = process.memory(); // in bytes
    let cpu_usage = process.cpu_usage(); // in %

    // Get database size
    let db_size = match db::get_database_size(&app_handle) {
        Ok((size, _)) => size,
        Err(_) => 0,
    };

    // Get executable size
    let exe_path = env::current_exe().map_err(|e| e.to_string())?;
    let exe_size = fs::metadata(exe_path).map(|m| m.len()).unwrap_or(0);

    Ok(AppMetrics {
        total_disk_size: db_size + exe_size,
        ram_usage,
        cpu_usage,
    })
}

#[tauri::command]
async fn reset_database(
    db_pool: tauri::State<'_, DbPool>,
    process_accumulators: tauri::State<'_, ProcessAccumulatorsState>,
    reset_signal: tauri::State<'_, ResetSignal>,
    app_handle: tauri::AppHandle,
) -> Result<ResetDatabaseResponse, String> {
    // Reset database with size info
    let pool_opt = {
        let guard = db_pool.0.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };

    let (db_size_before, db_size_after) = if let Some(pool) = pool_opt {
        db::reset_database_with_size(&pool, &app_handle)
            .await
            .map_err(|e| format!("Database error: {}", e))?
    } else {
        return Err("Database not initialized".to_string());
    };

    // Reset process I/O accumulators
    process_monitor::reset_accumulators(&process_accumulators.0);

    // Signal monitors to reset their baselines
    reset_signal.0.store(true, Ordering::Relaxed);

    Ok(ResetDatabaseResponse {
        db_size_before,
        db_size_after,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create shared pool state
    let db_pool = DbPool(Arc::new(Mutex::new(None)));
    let db_pool_clone = Arc::clone(&db_pool.0);

    // Create shared process accumulators state
    let process_accumulators = process_monitor::create_accumulators();
    let process_accumulators_state = ProcessAccumulatorsState(Arc::clone(&process_accumulators));

    // Create shared reset signal
    let reset_signal = Arc::new(AtomicBool::new(false));
    let reset_signal_state = ResetSignal(Arc::clone(&reset_signal));
    let reset_signal_monitor = Arc::clone(&reset_signal);

    // Create shared shutdown signal
    let shutdown_signal = Arc::new(AtomicBool::new(false));
    let shutdown_signal_state = ShutdownSignal(Arc::clone(&shutdown_signal));
    let shutdown_signal_monitor = Arc::clone(&shutdown_signal);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_pool)
        .manage(process_accumulators_state)
        .manage(reset_signal_state)
        .manage(shutdown_signal_state)
        .manage(SystemState(Mutex::new(System::new_all())))
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let pool_for_setup = Arc::clone(&db_pool_clone);
            let accumulators_for_monitor = Arc::clone(&process_accumulators);

            // Setup window close event to trigger graceful shutdown
            let main_window = app.get_webview_window("main");
            if let Some(window) = main_window {
                let shutdown_clone = Arc::clone(&shutdown_signal_monitor);
                let accumulators_clone = Arc::clone(&process_accumulators);
                let pool_clone = Arc::clone(&db_pool_clone);

                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        println!("[App] Close requested, triggering shutdown signal.");
                        shutdown_clone.store(true, Ordering::Relaxed);

                        // Save process stats to DB
                        if let Ok(pool_guard) = pool_clone.lock() {
                            if let Some(pool) = pool_guard.as_ref() {
                                let stats =
                                    process_monitor::get_aggregated_stats(&accumulators_clone);
                                if !stats.is_empty() {
                                    println!(
                                        "[App] Saving {} process records to history...",
                                        stats.len()
                                    );
                                    // We need to block here to ensure save completes, but we can't await in sync closure
                                    // So we spawn a thread and join it, or just use block_on if available.
                                    // Since we are in a sync context and need to block exit, we use a new runtime or block_on.
                                    // However, tauri's async_runtime::block_on is available.
                                    let pool_ref = pool.clone();
                                    tauri::async_runtime::block_on(async move {
                                        if let Err(e) =
                                            db::update_process_history(&pool_ref, stats).await
                                        {
                                            eprintln!(
                                                "[App] Failed to save process history: {}",
                                                e
                                            );
                                        } else {
                                            println!("[App] Process history saved successfully.");
                                        }
                                    });
                                }
                            }
                        }

                        // Give monitor time to flush before exit
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                });
            }

            // Initialize disk monitoring
            tauri::async_runtime::spawn(async move {
                match db::init_db(&app_handle).await {
                    Ok(pool) => {
                        // Store pool in state
                        if let Ok(mut pool_guard) = pool_for_setup.lock() {
                            *pool_guard = Some(pool.clone());
                        }
                        monitor::init_monitoring(
                            pool,
                            app_handle,
                            reset_signal_monitor,
                            shutdown_signal_monitor,
                            accumulators_for_monitor,
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize database: {}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_alltime_totals,
            save_session_to_alltime,
            get_database_size,
            get_app_metrics,
            reset_database,
            get_process_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

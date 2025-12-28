// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use sqlx::{Pool, Sqlite};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{Manager, Emitter};
use tokio::sync::Notify;

mod db;
mod models;
pub mod db_cleanup;
pub mod scheduled_tasks;
pub mod monitor;
pub mod perf_counters;
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
pub struct ShutdownNotify(pub Arc<Notify>);

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

// save_session_to_alltime command removed as it was causing double counting.
// Monitor handles real-time updates to process_history.

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
async fn get_process_history_totals(
    db_pool: tauri::State<'_, DbPool>,
) -> Result<AllTimeTotals, String> {
    let pool_opt = {
        let guard = db_pool.0.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };

    if let Some(pool) = pool_opt {
        let history = db::get_process_history(&pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        let total_read: u64 = history.values().map(|(r, _)| r).sum();
        let total_write: u64 = history.values().map(|(_, w)| w).sum();

        Ok(AllTimeTotals {
            read_bytes: total_read,
            write_bytes: total_write,
        })
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

    // Signal monitors to reset their baselines (includes process accumulators)
    reset_signal.0.store(true, Ordering::Relaxed);

    // Emit reset notification to frontend to refresh data
    let _ = app_handle.emit("database-reset", ());

    Ok(ResetDatabaseResponse {
        db_size_before,
        db_size_after,
    })
}

#[tauri::command]
async fn optimize_database(
    db_pool: tauri::State<'_, DbPool>,
) -> Result<serde_json::Value, String> {
    let pool_opt = {
        let guard = db_pool.0.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };

    if let Some(pool) = pool_opt {
        // Run cleanup with default retention policy (30 days)
        let policy = db_cleanup::RetentionPolicy::default();
        
        let cleaned_records = db_cleanup::cleanup_old_data(&pool, &policy)
            .await
            .map_err(|e| format!("Cleanup error: {}", e))?;

        // Get database size before VACUUM
        let db_size_before = match std::fs::metadata(
            std::path::PathBuf::from(&std::env::var("APPDATA").unwrap_or_default())
                .join("driveanalizer")
                .join("drive_analytics.db")
        ) {
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        };

        // Run VACUUM to reclaim space
        db_cleanup::vacuum_database(&pool)
            .await
            .map_err(|e| format!("VACUUM error: {}", e))?;

        // Run ANALYZE for query optimization
        db_cleanup::analyze_database(&pool)
            .await
            .map_err(|e| format!("ANALYZE error: {}", e))?;

        // Get database size after VACUUM
        let db_size_after = match std::fs::metadata(
            std::path::PathBuf::from(&std::env::var("APPDATA").unwrap_or_default())
                .join("driveanalizer")
                .join("drive_analytics.db")
        ) {
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        };

        let freed_bytes = db_size_before.saturating_sub(db_size_after);

        Ok(serde_json::json!({
            "cleaned_records": cleaned_records,
            "freed_bytes": freed_bytes,
            "db_size_before": db_size_before,
            "db_size_after": db_size_after,
        }))
    } else {
        Err("Database not initialized".to_string())
    }
}
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

    // Create shared shutdown notify
    let shutdown_notify = Arc::new(Notify::new());
    let shutdown_notify_state = ShutdownNotify(Arc::clone(&shutdown_notify));
    let shutdown_notify_monitor = Arc::clone(&shutdown_notify);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_pool)
        .manage(process_accumulators_state)
        .manage(reset_signal_state)
        .manage(shutdown_signal_state)
        .manage(shutdown_notify_state)
        .manage(SystemState(Mutex::new(System::new_all())))
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let pool_for_setup = Arc::clone(&db_pool_clone);
            let accumulators_for_monitor = Arc::clone(&process_accumulators);

            // Setup window close event to trigger graceful shutdown
            let main_window = app.get_webview_window("main");
            if let Some(window) = main_window {
                let shutdown_clone = Arc::clone(&shutdown_signal_monitor);
                let shutdown_notify_monitor = Arc::clone(&shutdown_notify_monitor);

                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        println!("[App] Close requested, triggering shutdown signal.");
                        shutdown_clone.store(true, Ordering::Relaxed);
                        shutdown_notify_monitor.notify_waiters();

                        // Give monitor time to flush buffer
                        std::thread::sleep(std::time::Duration::from_millis(1000));

                        // Final sleep to ensure everything is written
                        std::thread::sleep(std::time::Duration::from_millis(200));
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
                        
                        // Start scheduled tasks
                        let pool_for_cleanup = Arc::new(pool.clone());
                        let pool_for_analyze = Arc::new(pool.clone());
                        let pool_for_checkpoint = Arc::new(pool.clone());
                        
                        // Spawn cleanup scheduler (24 hours)
                        tauri::async_runtime::spawn(
                            scheduled_tasks::start_cleanup_scheduler(pool_for_cleanup)
                        );
                        
                        // Spawn analyze scheduler (7 days)
                        tauri::async_runtime::spawn(
                            scheduled_tasks::start_analyze_scheduler(pool_for_analyze)
                        );
                        
                        // Spawn WAL checkpoint scheduler (6 hours)
                        tauri::async_runtime::spawn(
                            scheduled_tasks::start_wal_checkpoint_scheduler(pool_for_checkpoint)
                        );
                        
                        println!("[Schedulers] All database maintenance schedulers started");
                        
                        monitor::init_monitoring(
                            pool,
                            app_handle,
                            reset_signal_monitor,
                            shutdown_signal_monitor,
                            shutdown_notify_monitor,
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
            get_database_size,
            get_app_metrics,
            reset_database,
            optimize_database,
            get_process_history,
            get_process_history_totals
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

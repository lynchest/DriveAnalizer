use crate::db;
use crate::models::DiskStat;
use crate::perf_counters;
use crate::process_monitor::{ProcessAccumulators, ProcessMonitor};
use sqlx::{Pool, Sqlite};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

pub fn init_monitoring(
    pool: Pool<Sqlite>,
    app: AppHandle,
    reset_signal: Arc<AtomicBool>,
    shutdown_signal: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
    accumulators: ProcessAccumulators,
) {
    tauri::async_runtime::spawn(async move {
        let mut buffer: Vec<DiskStat> = Vec::new();
        let mut process_monitor = ProcessMonitor::new(accumulators);
        
        let mut session_read_bytes: u64 = 0;
        let mut session_write_bytes: u64 = 0;

        let mut tick_count: u64 = 0;
        let mut last_flush = std::time::Instant::now();
        let mut cached_perf_metrics: (f64, f64) = (100.0, 0.0);

        loop {
            // Shutdown check
            if shutdown_signal.load(Ordering::Relaxed) {
                println!("[Monitor] Shutdown signal received. Flushing remaining buffer.");
                if !buffer.is_empty() {
                    if let Err(e) = db::insert_stats_batch(&pool, &buffer).await {
                        eprintln!("[Monitor] Final DB Flush Error: {}", e);
                    } else {
                        println!("[Monitor] Successfully flushed {} records.", buffer.len());
                    }
                }
                break;
            }

            // Reset check
            if reset_signal.load(Ordering::Relaxed) {
                println!("[Monitor] Reset signal received. Resetting baselines.");
                session_read_bytes = 0;
                session_write_bytes = 0;
                buffer.clear();
                last_flush = std::time::Instant::now();
                process_monitor.reset();
                reset_signal.store(false, Ordering::Relaxed);
            }

            // 1. Disk performance metrics (every 5 ticks)
            if tick_count % 5 == 0 {
                if let Ok(metrics) =
                    tokio::task::spawn_blocking(perf_counters::get_disk_perf_metrics_safe).await
                {
                    cached_perf_metrics = metrics;
                }
            }
            let (idle, queue) = cached_perf_metrics;

            // 2. Update processes and get deltas
            let (tick_read_delta, tick_write_delta) = process_monitor.update();

            // Update session totals
            session_read_bytes = session_read_bytes.saturating_add(tick_read_delta);
            session_write_bytes = session_write_bytes.saturating_add(tick_write_delta);

            let stat = DiskStat {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0),
                read_bytes: session_read_bytes,
                write_bytes: session_write_bytes,
                read_speed: tick_read_delta,
                write_speed: tick_write_delta,
                idle_time: idle,
                queue_depth: queue,
            };

            // Emit Dashboard Metrics
            if let Err(e) = app.emit("disk-metrics", &stat) {
                eprintln!("[Monitor] Failed to emit event: {}", e);
            }

            // Emit Top Processes (Every tick)
            tick_count += 1;
            // if tick_count % 2 == 0 {
            let process_stats = process_monitor.get_top_processes();
            if let Err(e) = app.emit("top-processes", &process_stats) {
                eprintln!("[Monitor] Failed to emit top-processes: {}", e);
            }
            // }

            // Unified Flush - Every 10 seconds
            buffer.push(stat.clone());
            if buffer.len() >= 60 || last_flush.elapsed() >= std::time::Duration::from_secs(10) {
                // 1. Flush Disk Stats
                if !buffer.is_empty() {
                    if let Err(e) = db::insert_stats_batch(&pool, &buffer).await {
                        eprintln!("[Monitor] DB Error: {}", e);
                    }
                    buffer.clear();
                }

                // 2. Flush Process History Deltas
                let deltas = process_monitor.get_deltas_for_db();
                if !deltas.is_empty() {
                    let pool_clone = pool.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = db::update_process_history(&pool_clone, deltas).await {
                            eprintln!("[Monitor] Failed to auto-save process history: {}", e);
                        }
                    });
                }

                // Periodic cleanup - every hour
                if tick_count % 3600 == 0 && tick_count > 0 {
                    let pool_cleanup = pool.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = db::cleanup_old_data(&pool_cleanup, 7).await;
                    });
                }

                last_flush = std::time::Instant::now();
            }

            tokio::select! {
                _ = sleep(Duration::from_secs(1)) => {}
                _ = shutdown_notify.notified() => {
                    println!("[Monitor] Notification received. Waking up.");
                }
            }
        }
    });
}

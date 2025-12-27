use crate::db;
use crate::models::DiskStat;
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};
use sysinfo::{ProcessesToUpdate, System};
use std::collections::HashSet;

#[derive(Deserialize, Debug)]
struct PsMetrics {
    it: Option<f64>,
    qd: Option<f64>,
}

pub fn init_monitoring(pool: Pool<Sqlite>, app: AppHandle, reset_signal: Arc<AtomicBool>, shutdown_signal: Arc<AtomicBool>) {
    tauri::async_runtime::spawn(async move {
        let mut buffer: Vec<DiskStat> = Vec::new();

        // Sysinfo setup for reliable I/O tracking
        let mut sys = System::new();
        let mut known_pids: HashSet<u32> = HashSet::new();
        
        // Session Totals
        let mut session_read_bytes: u64 = 0;
        let mut session_write_bytes: u64 = 0;

        loop {
            // Check for shutdown signal first to gracefully exit
            if shutdown_signal.load(Ordering::Relaxed) {
                println!("[Monitor] Shutdown signal received. Flushing remaining buffer.");
                if !buffer.is_empty() {
                    if let Err(e) = db::insert_stats_batch(&pool, &buffer).await {
                        eprintln!("[Monitor] Final DB Flush Error: {}", e);
                    } else {
                        println!("[Monitor] Successfully flushed {} records before shutdown.", buffer.len());
                    }
                }
                break;
            }

            // Check for reset signal
            if reset_signal.load(Ordering::Relaxed) {
                println!("[Monitor] Reset signal received. Resetting baselines.");
                session_read_bytes = 0;
                session_write_bytes = 0;
                known_pids.clear();
                buffer.clear();
                reset_signal.store(false, Ordering::Relaxed);
            }

            // 1. Fetch Idle Time and Queue Depth from PowerShell (concurrently)
            // We use PowerShell only for these metrics as sysinfo doesn't provide them.
            // If this fails, we default to 0.
            let ps_future = tokio::process::Command::new("powershell")
                .args(&["-NoProfile", "-Command", "
                    $p1 = Get-CimInstance Win32_PerfFormattedData_PerfDisk_PhysicalDisk -Filter \"Name = '_Total'\" | Select-Object -First 1;
                    @{
                        it = [double]$p1.PercentIdleTime;
                        qd = [double]$p1.AvgDiskQueueLength;
                    } | ConvertTo-Json -Compress
                "])
                .output();

            // 2. Refresh processes and calculate I/O using sysinfo
            // This is more robust than Windows Performance Counters which can be broken/stuck.
            sys.refresh_processes(ProcessesToUpdate::All);
            
            let mut current_read_speed: u64 = 0;
            let mut current_write_speed: u64 = 0;
            
            let active_pids: HashSet<u32> = sys.processes().keys().map(|p| p.as_u32()).collect();
            
            for (pid, process) in sys.processes() {
                let pid_u32 = pid.as_u32();
                let disk_usage = process.disk_usage();
                
                // Skip new processes for one tick to avoid initial spikes
                if !known_pids.contains(&pid_u32) {
                    known_pids.insert(pid_u32);
                    continue;
                }
                
                current_read_speed += disk_usage.read_bytes;
                current_write_speed += disk_usage.written_bytes;
            }
            
            // Cleanup known_pids
            known_pids.retain(|pid| active_pids.contains(pid));
            
            // Update totals
            session_read_bytes += current_read_speed;
            session_write_bytes += current_write_speed;

            // 3. Get PowerShell result
            let output = ps_future.await;
            let (idle, queue) = match output {
                Ok(out) if out.status.success() => {
                    let json_str = String::from_utf8_lossy(&out.stdout);
                    if let Ok(m) = serde_json::from_str::<PsMetrics>(&json_str) {
                        (m.it.unwrap_or(0.0), m.qd.unwrap_or(0.0))
                    } else {
                        (0.0, 0.0)
                    }
                }
                _ => (0.0, 0.0),
            };

            let stat = DiskStat {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64(),
                read_bytes: session_read_bytes,
                write_bytes: session_write_bytes,
                read_speed: current_read_speed,
                write_speed: current_write_speed,
                idle_time: idle,
                queue_depth: queue,
            };

            // Emit to Frontend
            if let Err(e) = app.emit("disk-metrics", &stat) {
                eprintln!("[Monitor] Failed to emit event: {}", e);
            }

            // Database Batching
            buffer.push(stat.clone());
            // Save every 60 seconds for optimal performance
            if buffer.len() >= 60 {
                if let Err(e) = db::insert_stats_batch(&pool, &buffer).await {
                    eprintln!("[Monitor] DB Error: {}", e);
                }
                
                buffer.clear();
            }

            sleep(Duration::from_millis(1000)).await;
        }
    });
}

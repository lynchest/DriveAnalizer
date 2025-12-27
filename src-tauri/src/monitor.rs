use crate::db;
use crate::models::{DiskStat, ProcessIOStat};
use crate::process_monitor::{ProcessAccumulators, ProcessIOAccumulator};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use sysinfo::{ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

#[derive(Deserialize, Debug)]
struct PsMetrics {
    it: Option<f64>,
    qd: Option<f64>,
}

pub fn init_monitoring(
    pool: Pool<Sqlite>,
    app: AppHandle,
    reset_signal: Arc<AtomicBool>,
    shutdown_signal: Arc<AtomicBool>,
    accumulators: ProcessAccumulators,
) {
    tauri::async_runtime::spawn(async move {
        let mut buffer: Vec<DiskStat> = Vec::new();

        // Sysinfo setup for reliable I/O tracking
        let mut sys = System::new();
        let mut known_pids: HashSet<u32> = HashSet::new();

        // Session Totals
        let mut session_read_bytes: u64 = 0;
        let mut session_write_bytes: u64 = 0;

        // Counter for lower-frequency process updates
        let mut tick_count: u64 = 0;

        // Dead Process History (Name -> (Read, Write))
        let mut dead_process_history: HashMap<String, (u64, u64)> = HashMap::new();

        loop {
            // Check for shutdown signal first to gracefully exit
            if shutdown_signal.load(Ordering::Relaxed) {
                println!("[Monitor] Shutdown signal received. Flushing remaining buffer.");
                if !buffer.is_empty() {
                    if let Err(e) = db::insert_stats_batch(&pool, &buffer).await {
                        eprintln!("[Monitor] Final DB Flush Error: {}", e);
                    } else {
                        println!(
                            "[Monitor] Successfully flushed {} records before shutdown.",
                            buffer.len()
                        );
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
                dead_process_history.clear();
                // Accumulators are reset by the command, but we should clear known_pids here
                // to sync with the new empty state.
                reset_signal.store(false, Ordering::Relaxed);
            }

            // 1. Fetch Idle Time and Queue Depth from PowerShell (concurrently)
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
            sys.refresh_processes(ProcessesToUpdate::All);

            let mut current_read_speed: u64 = 0;
            let mut current_write_speed: u64 = 0;

            let active_pids: HashSet<u32> = sys.processes().keys().map(|p| p.as_u32()).collect();

            // Lock accumulators
            if let Ok(mut acc_guard) = accumulators.lock() {
                for (pid, process) in sys.processes() {
                    let pid_u32 = pid.as_u32();
                    let disk_usage = process.disk_usage();
                    let name = process.name().to_string_lossy().to_string();

                    // Skip new processes for one tick to avoid initial spikes
                    if !known_pids.contains(&pid_u32) {
                        known_pids.insert(pid_u32);
                        // Initialize accumulator
                        acc_guard.entry(pid_u32).or_insert(ProcessIOAccumulator {
                            name,
                            read_bytes: 0,
                            write_bytes: 0,
                        });
                        continue;
                    }

                    // Add to Totals (Dashboard)
                    current_read_speed += disk_usage.read_bytes;
                    current_write_speed += disk_usage.written_bytes;

                    // Add to Accumulators (Top Processes)
                    let acc = acc_guard.entry(pid_u32).or_insert(ProcessIOAccumulator {
                        name,
                        read_bytes: 0,
                        write_bytes: 0,
                    });
                    acc.read_bytes += disk_usage.read_bytes;
                    acc.write_bytes += disk_usage.written_bytes;
                }

                // Identify and move dead processes to history
                let dead_pids: Vec<u32> = acc_guard
                    .keys()
                    .filter(|pid| !active_pids.contains(pid))
                    .cloned()
                    .collect();

                for pid in dead_pids {
                    if let Some(acc) = acc_guard.remove(&pid) {
                        if acc.read_bytes > 0 || acc.write_bytes > 0 {
                            let entry = dead_process_history.entry(acc.name).or_insert((0, 0));
                            entry.0 += acc.read_bytes;
                            entry.1 += acc.write_bytes;
                        }
                    }
                }

                // Cleanup known_pids
                known_pids.retain(|pid| active_pids.contains(pid));
            } else {
                eprintln!("[Monitor] Failed to lock accumulators");
            }

            // Update session totals
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

            // Emit Dashboard Metrics
            if let Err(e) = app.emit("disk-metrics", &stat) {
                eprintln!("[Monitor] Failed to emit event: {}", e);
            }

            // Emit Top Processes (Every 2nd tick = 2 seconds)
            tick_count += 1;
            if tick_count % 2 == 0 {
                // Generate Top 50 List
                // Start with dead process history
                let mut grouped: HashMap<String, (Option<String>, u64, u64)> = HashMap::new();
                for (name, (r, w)) in &dead_process_history {
                    grouped.insert(name.clone(), (None, *r, *w));
                }

                if let Ok(acc_guard) = accumulators.lock() {
                    for (pid, process) in sys.processes() {
                        let pid_u32 = pid.as_u32();
                        if let Some(acc) = acc_guard.get(&pid_u32) {
                            if acc.read_bytes == 0 && acc.write_bytes == 0 {
                                continue;
                            }

                            let name = process.name().to_string_lossy().to_string();
                            let exe_path = process.exe().map(|p| p.to_string_lossy().to_string());

                            let entry = grouped.entry(name).or_insert((exe_path, 0, 0));
                            entry.1 += acc.read_bytes;
                            entry.2 += acc.write_bytes;
                        }
                    }
                }

                let mut process_stats: Vec<ProcessIOStat> = grouped
                    .into_iter()
                    .map(
                        |(name, (exe_path, read_bytes, write_bytes))| ProcessIOStat {
                            pid: 0,
                            name,
                            exe_path,
                            read_bytes,
                            write_bytes,
                            total_bytes: read_bytes + write_bytes,
                        },
                    )
                    .collect();

                process_stats.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
                process_stats.truncate(50);

                if let Err(e) = app.emit("top-processes", &process_stats) {
                    eprintln!("[Monitor] Failed to emit top-processes: {}", e);
                }
            }

            // Database Batching
            buffer.push(stat.clone());
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

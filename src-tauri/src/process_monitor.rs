use crate::models::ProcessIOStat;
use std::collections::{HashMap, HashSet};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};
use sysinfo::{ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

/// Holds cumulative I/O data for processes across the session
#[derive(Clone)]
pub struct ProcessIOAccumulator {
    name: String, // Store name to persist even if process dies
    read_bytes: u64,
    write_bytes: u64,
}

/// Global state for process accumulators - allows external reset
pub type ProcessAccumulators = Arc<Mutex<HashMap<u32, ProcessIOAccumulator>>>;

/// Creates a new shared accumulator state
pub fn create_accumulators() -> ProcessAccumulators {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Resets all process I/O accumulators
pub fn reset_accumulators(accumulators: &ProcessAccumulators) {
    if let Ok(mut acc) = accumulators.lock() {
        acc.clear();
    }
}

/// Gets aggregated stats by process name for saving to DB
pub fn get_aggregated_stats(accumulators: &ProcessAccumulators) -> HashMap<String, (u64, u64)> {
    let mut aggregated: HashMap<String, (u64, u64)> = HashMap::new();

    if let Ok(acc_guard) = accumulators.lock() {
        for (_, acc) in acc_guard.iter() {
            if acc.read_bytes == 0 && acc.write_bytes == 0 {
                continue;
            }
            
            let entry = aggregated.entry(acc.name.clone()).or_insert((0, 0));
            entry.0 += acc.read_bytes;
            entry.1 += acc.write_bytes;
        }
    }
    aggregated
}

pub fn init_process_monitoring(app: AppHandle, accumulators: ProcessAccumulators, reset_signal: Arc<AtomicBool>) {
    tauri::async_runtime::spawn(async move {
        // System instance for process information
        let mut sys = System::new();
        let mut known_pids: HashSet<u32> = HashSet::new();

        loop {
            // Check for reset signal
            if reset_signal.load(Ordering::Relaxed) {
                println!("[ProcessMonitor] Reset signal received. Clearing known PIDs.");
                known_pids.clear();
                // Note: accumulators are cleared by the command handler calling reset_accumulators
                // We just need to clear known_pids so we skip the next tick for everyone
                // to avoid spikes if sysinfo returns cumulative data.
                reset_signal.store(false, Ordering::Relaxed);
            }

            // Refresh all processes
            sys.refresh_processes(ProcessesToUpdate::All);

            // Lock accumulators for this iteration
            let lock_success = {
                match accumulators.lock() {
                    Ok(mut acc_guard) => {
                        // Collect I/O data from all processes
                        for (pid, process) in sys.processes() {
                            let pid_u32 = pid.as_u32();
                            let disk_usage = process.disk_usage();

                            // If this is a new process we haven't seen before (or after reset),
                            // we skip adding its usage this tick. This prevents initial spikes
                            // if sysinfo returns cumulative lifetime usage on the first call.
                            if !known_pids.contains(&pid_u32) {
                                known_pids.insert(pid_u32);
                                // Initialize accumulator but don't add current usage yet
                                acc_guard.entry(pid_u32).or_insert(ProcessIOAccumulator {
                                    name: process.name().to_string_lossy().to_string(),
                                    read_bytes: 0,
                                    write_bytes: 0,
                                });
                                continue;
                            }

                            // Get or create accumulator for this process
                            let acc = acc_guard.entry(pid_u32).or_insert(ProcessIOAccumulator {
                                name: process.name().to_string_lossy().to_string(),
                                read_bytes: 0,
                                write_bytes: 0,
                            });

                            // Accumulate bytes (disk_usage returns bytes since last refresh)
                            acc.read_bytes += disk_usage.read_bytes;
                            acc.write_bytes += disk_usage.written_bytes;
                        }

                        // Clean up dead processes from accumulator
                        let active_pids: std::collections::HashSet<u32> =
                            sys.processes().keys().map(|p| p.as_u32()).collect();
                        acc_guard.retain(|pid, _| active_pids.contains(pid));
                        
                        // Also clean up known_pids to prevent memory leak
                        known_pids.retain(|pid| active_pids.contains(pid));
                        
                        true
                    }
                    Err(e) => {
                        eprintln!("[ProcessMonitor] Failed to lock accumulators: {}", e);
                        false
                    }
                }
            };

            if !lock_success {
                sleep(Duration::from_millis(2000)).await;
                continue;
            }

            // Group processes by name and aggregate their I/O
            let mut grouped: HashMap<String, (Option<String>, u64, u64)> = HashMap::new();

            // Re-lock for reading
            if let Ok(acc_guard) = accumulators.lock() {
                for (pid, process) in sys.processes() {
                    let pid_u32 = pid.as_u32();
                    if let Some(acc) = acc_guard.get(&pid_u32) {
                        // Skip processes with no I/O activity
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

            // Convert grouped data to ProcessIOStat list
            let mut process_stats: Vec<ProcessIOStat> = grouped
                .into_iter()
                .map(
                    |(name, (exe_path, read_bytes, write_bytes))| ProcessIOStat {
                        pid: 0, // Not applicable for grouped processes
                        name,
                        exe_path,
                        read_bytes,
                        write_bytes,
                        total_bytes: read_bytes + write_bytes,
                    },
                )
                .collect();

            // Sort by total bytes descending and take top 50
            process_stats.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
            process_stats.truncate(50);

            // Emit to frontend (emit even if empty after reset)
            if let Err(e) = app.emit("top-processes", &process_stats) {
                eprintln!("[ProcessMonitor] Failed to emit event: {}", e);
            }

            // Update every 2 seconds (balanced performance)
            sleep(Duration::from_millis(2000)).await;
        }
    });
}

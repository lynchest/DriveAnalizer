use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use sysinfo::{ProcessesToUpdate, System};
use crate::models::ProcessIOStat;

#[derive(Clone)]
pub struct ProcessIOAccumulator {
    pub name: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

pub type ProcessAccumulators = Arc<Mutex<HashMap<u32, ProcessIOAccumulator>>>;

pub fn create_accumulators() -> ProcessAccumulators {
    Arc::new(Mutex::new(HashMap::new()))
}

pub struct ProcessMonitor {
    sys: System,
    dead_process_history: HashMap<String, (u64, u64)>,
    last_process_snapshot: HashMap<String, (u64, u64)>,
    accumulators: ProcessAccumulators,
    last_seen_by_pid: HashMap<u32, (u64, u64)>,
}

impl ProcessMonitor {
    pub fn new(accumulators: ProcessAccumulators) -> Self {
        Self {
            sys: System::new(),
            dead_process_history: HashMap::new(),
            last_process_snapshot: HashMap::new(),
            accumulators,
            last_seen_by_pid: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        self.dead_process_history.clear();
        self.last_process_snapshot.clear();
        self.last_seen_by_pid.clear();
        if let Ok(mut acc) = self.accumulators.lock() {
            acc.clear();
        }
    }

    pub fn update(&mut self) -> (u64, u64) {
        self.sys.refresh_processes(ProcessesToUpdate::All);
        let mut tick_read_delta: u64 = 0;
        let mut tick_write_delta: u64 = 0;

        let active_pids: HashSet<u32> = self
            .sys
            .processes()
            .keys()
            .map(|p| p.as_u32())
            .collect();

        if let Ok(mut acc_guard) = self.accumulators.lock() {
            for (pid, process) in self.sys.processes() {
                let pid_u32 = pid.as_u32();
                let disk_usage = process.disk_usage();

                // sysinfo::Process::disk_usage() returns cumulative bytes since the process
                // started. We must compute per-tick deltas to avoid double counting.
                let current_read = disk_usage.read_bytes;
                let current_write = disk_usage.written_bytes;

                let (r_delta, w_delta) = match self.last_seen_by_pid.get_mut(&pid_u32) {
                    Some((prev_r, prev_w)) => {
                        let r = current_read.saturating_sub(*prev_r);
                        let w = current_write.saturating_sub(*prev_w);
                        *prev_r = current_read;
                        *prev_w = current_write;
                        (r, w)
                    }
                    None => {
                        // New to our monitor session: establish baseline; count 0 for this tick.
                        self.last_seen_by_pid.insert(pid_u32, (current_read, current_write));
                        (0, 0)
                    }
                };

                let acc = acc_guard.entry(pid_u32).or_insert_with(|| ProcessIOAccumulator {
                    name: process.name().to_string_lossy().to_string(),
                    read_bytes: 0,
                    write_bytes: 0,
                });

                // Keep name fresh (helps with long-running processes that change name/exe)
                acc.name = process.name().to_string_lossy().to_string();

                if r_delta > 0 || w_delta > 0 {
                    acc.read_bytes = acc.read_bytes.saturating_add(r_delta);
                    acc.write_bytes = acc.write_bytes.saturating_add(w_delta);
                    tick_read_delta = tick_read_delta.saturating_add(r_delta);
                    tick_write_delta = tick_write_delta.saturating_add(w_delta);
                }
            }

            // Handle dead processes (present in our maps but no longer active)
            let dead_pids: Vec<u32> = self
                .last_seen_by_pid
                .keys()
                .filter(|pid| !active_pids.contains(pid))
                .cloned()
                .collect();

            for pid in dead_pids {
                self.last_seen_by_pid.remove(&pid);
                if let Some(acc) = acc_guard.remove(&pid) {
                    if acc.read_bytes > 0 || acc.write_bytes > 0 {
                        let entry = self.dead_process_history.entry(acc.name).or_insert((0, 0));
                        entry.0 = entry.0.saturating_add(acc.read_bytes);
                        entry.1 = entry.1.saturating_add(acc.write_bytes);
                    }
                }
            }
        }

        (tick_read_delta, tick_write_delta)
    }

    pub fn get_top_processes(&self) -> Vec<ProcessIOStat> {
        let mut grouped: HashMap<String, (Option<String>, u64, u64)> = HashMap::new();

        for (name, (r, w)) in &self.dead_process_history {
            grouped.insert(name.clone(), (None, *r, *w));
        }

        if let Ok(acc_guard) = self.accumulators.lock() {
            for (pid, process) in self.sys.processes() {
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

        let mut stats: Vec<ProcessIOStat> = grouped
            .into_iter()
            .map(|(name, (exe_path, r, w))| ProcessIOStat {
                pid: 0,
                name,
                exe_path,
                read_bytes: r,
                write_bytes: w,
                total_bytes: r + w,
            })
            .collect();

        stats.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));

        // Calculate totals before truncation to handle "Others"
        let total_read: u64 = stats.iter().map(|s| s.read_bytes).sum();
        let total_write: u64 = stats.iter().map(|s| s.write_bytes).sum();

        if stats.len() > 50 {
            stats.truncate(50);

            let top_read: u64 = stats.iter().map(|s| s.read_bytes).sum();
            let top_write: u64 = stats.iter().map(|s| s.write_bytes).sum();

            let other_read = total_read.saturating_sub(top_read);
            let other_write = total_write.saturating_sub(top_write);

            if other_read > 0 || other_write > 0 {
                stats.push(ProcessIOStat {
                    pid: 0,
                    name: "Others".to_string(),
                    exe_path: None,
                    read_bytes: other_read,
                    write_bytes: other_write,
                    total_bytes: other_read + other_write,
                });
            }
        }

        stats
    }

    pub fn get_deltas_for_db(&mut self) -> HashMap<String, (u64, u64)> {
        let mut deltas: HashMap<String, (u64, u64)> = HashMap::new();

        // Aggregate current totals by process name across active + dead processes.
        // This avoids snapshot collisions when multiple PIDs share the same name.
        let mut current_totals: HashMap<String, (u64, u64)> = self.dead_process_history.clone();
        if let Ok(acc_guard) = self.accumulators.lock() {
            for acc in acc_guard.values() {
                let entry = current_totals.entry(acc.name.clone()).or_insert((0, 0));
                entry.0 = entry.0.saturating_add(acc.read_bytes);
                entry.1 = entry.1.saturating_add(acc.write_bytes);
            }
        }

        for (name, (cur_r, cur_w)) in current_totals {
            let snapshot = self
                .last_process_snapshot
                .entry(name.clone())
                .or_insert((0, 0));
            let r_delta = cur_r.saturating_sub(snapshot.0);
            let w_delta = cur_w.saturating_sub(snapshot.1);

            if r_delta > 0 || w_delta > 0 {
                deltas.insert(name, (r_delta, w_delta));
                snapshot.0 = cur_r;
                snapshot.1 = cur_w;
            }
        }

        deltas
    }
}

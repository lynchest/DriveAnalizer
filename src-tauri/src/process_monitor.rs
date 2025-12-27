use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ProcessIOAccumulator {
    pub name: String, // Store name to persist even if process dies
    pub read_bytes: u64,
    pub write_bytes: u64,
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

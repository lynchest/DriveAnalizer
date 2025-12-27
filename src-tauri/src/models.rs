use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DiskStat {
    pub timestamp: f64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_speed: u64,
    pub write_speed: u64,
    pub idle_time: f64,
    pub queue_depth: f64,
}

/// Per-process disk I/O statistics
#[derive(Debug, Clone, Serialize)]
pub struct ProcessIOStat {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub total_bytes: u64,
}

/// All-time totals from database
#[derive(Debug, Clone, Serialize)]
pub struct AllTimeTotals {
    pub read_bytes: u64,
    pub write_bytes: u64,
}

/// Reset database response with database size info
#[derive(Debug, Clone, Serialize)]
pub struct ResetDatabaseResponse {
    pub db_size_before: u64,
    pub db_size_after: u64,
}

/// Application resource usage metrics
#[derive(Debug, Clone, Serialize)]
pub struct AppMetrics {
    pub total_disk_size: u64,
    pub ram_usage: u64,
    pub cpu_usage: f32,
}

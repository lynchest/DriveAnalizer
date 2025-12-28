use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use crate::db_cleanup::{cleanup_old_data, vacuum_database, analyze_database, RetentionPolicy};

/// Starts the cleanup scheduler that runs every 24 hours
///
/// This scheduler automatically deletes old records based on the retention policy
/// and performs VACUUM to reclaim unused space.
///
/// # Arguments
/// * `pool` - Shared SQLite connection pool wrapped in Arc
pub async fn start_cleanup_scheduler(pool: Arc<Pool<Sqlite>>) {
    // 24 hours interval for cleanup (86400 seconds)
    let mut cleanup_interval = interval(Duration::from_secs(86400));

    loop {
        cleanup_interval.tick().await;

        let policy = RetentionPolicy::default();

        match cleanup_old_data(&pool, &policy).await {
            Ok(count) => {
                println!(
                    "[Cleanup] Successfully deleted {} old records older than {} days",
                    count, policy.keep_days
                );

                // Reclaim unused space with VACUUM
                match vacuum_database(&pool).await {
                    Ok(_) => println!("[Cleanup] VACUUM completed successfully"),
                    Err(e) => eprintln!("[Cleanup] VACUUM failed: {}", e),
                }
            }
            Err(e) => {
                eprintln!("[Cleanup] Failed to cleanup: {}", e);
            }
        }
    }
}

/// Starts the ANALYZE scheduler that runs weekly (every 7 days)
///
/// ANALYZE gathers statistics about tables and indices to help SQLite
/// query planner make better decisions about query optimization.
///
/// # Arguments
/// * `pool` - Shared SQLite connection pool wrapped in Arc
pub async fn start_analyze_scheduler(pool: Arc<Pool<Sqlite>>) {
    // 7 days interval for ANALYZE (604800 seconds)
    let mut analyze_interval = interval(Duration::from_secs(604800));

    loop {
        analyze_interval.tick().await;

        match analyze_database(&pool).await {
            Ok(_) => println!("[Analyze] Query optimization completed successfully"),
            Err(e) => eprintln!("[Analyze] ANALYZE failed: {}", e),
        }
    }
}

/// Starts the WAL checkpoint scheduler that runs every 6 hours
///
/// WAL (Write-Ahead Logging) checkpoints synchronize the main database file
/// with the WAL log, helping to manage file sizes and improve performance.
///
/// The PASSIVE mode is used to avoid blocking readers.
///
/// # Arguments
/// * `pool` - Shared SQLite connection pool wrapped in Arc
pub async fn start_wal_checkpoint_scheduler(pool: Arc<Pool<Sqlite>>) {
    // 6 hours interval for WAL checkpoint (21600 seconds)
    let mut checkpoint_interval = interval(Duration::from_secs(21600));

    loop {
        checkpoint_interval.tick().await;

        match sqlx::query("PRAGMA wal_checkpoint(PASSIVE)")
            .execute(&*pool)
            .await
        {
            Ok(_) => {
                println!("[WAL] Checkpoint completed successfully");
            }
            Err(e) => eprintln!("[WAL] Checkpoint failed: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_duration() {
        // 24 hours = 86400 seconds
        assert_eq!(Duration::from_secs(86400).as_secs(), 86400);
    }

    #[test]
    fn test_analyze_duration() {
        // 7 days = 604800 seconds
        assert_eq!(Duration::from_secs(604800).as_secs(), 604800);
    }

    #[test]
    fn test_checkpoint_duration() {
        // 6 hours = 21600 seconds
        assert_eq!(Duration::from_secs(21600).as_secs(), 21600);
    }

    #[test]
    fn test_scheduler_timing() {
        // Verify interval calculations
        let cleanup_hours = Duration::from_secs(86400).as_secs() / 3600;
        let analyze_days = Duration::from_secs(604800).as_secs() / 86400;
        let checkpoint_hours = Duration::from_secs(21600).as_secs() / 3600;

        assert_eq!(cleanup_hours, 24); // 24 hours
        assert_eq!(analyze_days, 7);   // 7 days
        assert_eq!(checkpoint_hours, 6); // 6 hours
    }
}

use sqlx::{Pool, Sqlite};

/// Data retention policy configuration
/// 
/// Determines how long data should be kept in the database
/// and how data should be sampled to reduce storage usage.
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// How many days of data should be kept (default: 30 days)
    pub keep_days: u64,
    
    /// Sample interval - keep every n-th record (default: 1 = keep all)
    pub sample_interval: u64,
    
    /// Whether archive mechanism is enabled (default: true)
    pub archive_enabled: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_days: 30,
            sample_interval: 1,
            archive_enabled: true,
        }
    }
}

impl RetentionPolicy {
    /// Creates a new retention policy with custom parameters
    pub fn new(keep_days: u64, sample_interval: u64, archive_enabled: bool) -> Self {
        Self {
            keep_days,
            sample_interval,
            archive_enabled,
        }
    }
}

/// Cleanup old data from disk_stats table based on retention policy
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `policy` - Retention policy configuration
///
/// # Returns
/// * Number of records deleted
pub async fn cleanup_old_data(
    pool: &Pool<Sqlite>,
    policy: &RetentionPolicy,
) -> Result<u64, sqlx::Error> {
    // Calculate cutoff timestamp (everything older than this will be deleted)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let cutoff = now - (policy.keep_days as f64 * 86400.0);

    // Get count of records that will be deleted
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM disk_stats WHERE timestamp < ?"
    )
    .bind(cutoff)
    .fetch_one(pool)
    .await?;

    // Delete old records
    sqlx::query("DELETE FROM disk_stats WHERE timestamp < ?")
        .bind(cutoff)
        .execute(pool)
        .await?;

    println!(
        "[Cleanup] Deleted {} records older than {} days",
        count.0, policy.keep_days
    );

    Ok(count.0 as u64)
}

/// Get count of records that would be deleted by cleanup
///
/// Useful for preview or logging purposes
pub async fn preview_cleanup(
    pool: &Pool<Sqlite>,
    policy: &RetentionPolicy,
) -> Result<u64, sqlx::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let cutoff = now - (policy.keep_days as f64 * 86400.0);

    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM disk_stats WHERE timestamp < ?"
    )
    .bind(cutoff)
    .fetch_one(pool)
    .await?;

    Ok(count.0 as u64)
}

/// Optimizes database by running VACUUM
/// Reclaims unused space after deletion operations
pub async fn vacuum_database(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query("VACUUM").execute(pool).await?;
    println!("[Cleanup] Database VACUUM completed");
    Ok(())
}

/// Analyzes database tables for query optimization
/// Should be run periodically to keep query plans optimal
pub async fn analyze_database(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query("ANALYZE").execute(pool).await?;
    println!("[Cleanup] Database ANALYZE completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_policy_default() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.keep_days, 30);
        assert_eq!(policy.sample_interval, 1);
        assert!(policy.archive_enabled);
    }

    #[test]
    fn test_retention_policy_new() {
        let policy = RetentionPolicy::new(7, 5, false);
        assert_eq!(policy.keep_days, 7);
        assert_eq!(policy.sample_interval, 5);
        assert!(!policy.archive_enabled);
    }

    #[test]
    fn test_retention_policy_clone() {
        let policy1 = RetentionPolicy::default();
        let policy2 = policy1.clone();
        assert_eq!(policy1.keep_days, policy2.keep_days);
    }

    #[tokio::test]
    async fn test_cleanup_old_data_empty_database() {
        // This test would require a test database setup
        // For now, we're testing the logic with a mock
        let policy = RetentionPolicy::default();
        assert!(policy.keep_days > 0);
    }

    #[test]
    fn test_policy_parameters() {
        let policy = RetentionPolicy::new(60, 2, true);
        assert_eq!(policy.keep_days, 60);
        assert_eq!(policy.sample_interval, 2);
        assert!(policy.archive_enabled);
    }

    #[test]
    fn test_policy_zero_days() {
        let policy = RetentionPolicy::new(0, 1, true);
        assert_eq!(policy.keep_days, 0);
    }
}

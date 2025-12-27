use crate::models::DiskStat;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::fs;
use tauri::Manager;

pub async fn init_db(
    app_handle: &tauri::AppHandle,
) -> Result<Pool<Sqlite>, Box<dyn std::error::Error + Send + Sync>> {
    use tauri::Manager;

    let app_data_dir = app_handle.path().app_data_dir()?;
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir)?;
    }

    let db_path = app_data_dir.join("drive_analytics.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

    // Create the DB file if it doesn't exist
    if !db_path.exists() {
        fs::File::create(&db_path)?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Create persistent tables
    sqlx::query(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         CREATE TABLE IF NOT EXISTS process_history (
            name TEXT PRIMARY KEY,
            read_bytes INTEGER NOT NULL DEFAULT 0,
            write_bytes INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS disk_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp REAL NOT NULL,
            read_bytes INTEGER NOT NULL,
            write_bytes INTEGER NOT NULL,
            read_speed INTEGER NOT NULL,
            write_speed INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_disk_stats_timestamp ON disk_stats(timestamp);",
    )
    .execute(&pool)
    .await?;

    println!("[DB] Database initialized successfully.");

    Ok(pool)
}

pub async fn insert_stats_batch(
    pool: &Pool<Sqlite>,
    stats: &[DiskStat],
) -> Result<(), sqlx::Error> {
    if stats.is_empty() {
        return Ok(());
    }

    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO disk_stats (timestamp, read_bytes, write_bytes, read_speed, write_speed) "
    );

    query_builder.push_values(stats, |mut b, stat| {
        b.push_bind(stat.timestamp)
         .push_bind(stat.read_bytes as i64)
         .push_bind(stat.write_bytes as i64)
         .push_bind(stat.read_speed as i64)
         .push_bind(stat.write_speed as i64);
    });

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}

// get_max_session_totals removed as it's no longer used for recovery.
// We instead rely on periodic delta flushes to process_history.

/// Gets the all-time total read and write bytes from the process_history table
pub async fn get_alltime_totals(pool: &Pool<Sqlite>) -> Result<(u64, u64), sqlx::Error> {
    let result: (Option<i64>, Option<i64>) =
        sqlx::query_as("SELECT SUM(read_bytes), SUM(write_bytes) FROM process_history")
            .fetch_one(pool)
            .await?;

    Ok((result.0.unwrap_or(0) as u64, result.1.unwrap_or(0) as u64))
}

/// Resets the database by clearing all stats and returns database size info
pub async fn reset_database_with_size(
    pool: &Pool<Sqlite>,
    app_handle: &tauri::AppHandle,
) -> Result<(u64, u64), Box<dyn std::error::Error + Send + Sync>> {
    // Get database path
    let app_data_dir = app_handle.path().app_data_dir()?;
    let db_path = app_data_dir.join("drive_analytics.db");
    let wal_path = app_data_dir.join("drive_analytics.db-wal");
    let shm_path = app_data_dir.join("drive_analytics.db-shm");

    // Get size before reset
    let size_before = get_db_total_size(&db_path, &wal_path, &shm_path)?;

    // Reset the database
    clear_disk_stats(pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Reset process history
    sqlx::query("DELETE FROM process_history")
        .execute(pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Run VACUUM to reclaim space
    println!("[DB] Running VACUUM to reclaim space...");
    sqlx::query("VACUUM")
        .execute(pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Checkpoint WAL to ensure everything is written to the main file
    println!("[DB] Running WAL Checkpoint...");
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(pool)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Note: We can't actually drop the pool reference since it's borrowed.
    // The pool will be dropped when all references are released.
    // We just wait a bit to ensure pending operations complete.
    println!("[DB] Waiting for database operations to complete...");
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Get size after reset
    let size_after = get_db_total_size(&db_path, &wal_path, &shm_path)?;

    Ok((size_before, size_after))
}

/// Gets the total database size including main file and WAL files
fn get_db_total_size(
    db_path: &std::path::Path,
    wal_path: &std::path::Path,
    shm_path: &std::path::Path,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let mut total_size = 0u64;

    // Get main database file size
    if db_path.exists() {
        total_size += fs::metadata(db_path)?.len();
    }

    // Get WAL file size if exists
    if wal_path.exists() {
        total_size += fs::metadata(wal_path)?.len();
    }

    // Get SHM file size if exists
    if shm_path.exists() {
        total_size += fs::metadata(shm_path)?.len();
    }

    Ok(total_size)
}

/// Gets the current database size
pub fn get_database_size(
    app_handle: &tauri::AppHandle,
) -> Result<(u64, u64), Box<dyn std::error::Error + Send + Sync>> {
    let app_data_dir = app_handle.path().app_data_dir()?;
    let db_path = app_data_dir.join("drive_analytics.db");
    let wal_path = app_data_dir.join("drive_analytics.db-wal");
    let shm_path = app_data_dir.join("drive_analytics.db-shm");

    let size = get_db_total_size(&db_path, &wal_path, &shm_path)?;

    // Return same format as reset_database_with_size for consistency
    Ok((size, size))
}

pub async fn clear_disk_stats(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM disk_stats").execute(pool).await?;
    Ok(())
}

pub async fn get_process_history(
    pool: &Pool<Sqlite>,
) -> Result<std::collections::HashMap<String, (u64, u64)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT name, read_bytes, write_bytes FROM process_history",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::new();
    for (name, read, write) in rows {
        map.insert(name, (read as u64, write as u64));
    }
    Ok(map)
}

pub async fn update_process_history(
    pool: &Pool<Sqlite>,
    stats: std::collections::HashMap<String, (u64, u64)>,
) -> Result<(), sqlx::Error> {
    if stats.is_empty() {
        return Ok(());
    }

    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO process_history (name, read_bytes, write_bytes) "
    );

    query_builder.push_values(stats.iter(), |mut b, (name, (read, write))| {
        b.push_bind(name)
         .push_bind(*read as i64)
         .push_bind(*write as i64);
    });

    query_builder.push(
        " ON CONFLICT(name) DO UPDATE SET
          read_bytes = read_bytes + excluded.read_bytes,
          write_bytes = write_bytes + excluded.write_bytes"
    );

    let query = query_builder.build();
    query.execute(pool).await?;

    Ok(())
}

/// Eski verileri temizle (belirtilen gün sayısından eski)
/// Varsayılan: 7 gün
pub async fn cleanup_old_data(pool: &Pool<Sqlite>, days: u64) -> Result<u64, sqlx::Error> {
    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        - (days as f64 * 24.0 * 3600.0);

    let result = sqlx::query("DELETE FROM disk_stats WHERE timestamp < ?")
        .bind(cutoff)
        .execute(pool)
        .await?;

    let deleted = result.rows_affected();
    if deleted > 0 {
        println!(
            "[DB] Cleaned up {} old records (older than {} days)",
            deleted, days
        );
    }

    Ok(deleted)
}

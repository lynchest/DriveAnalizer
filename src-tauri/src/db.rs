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

    sqlx::query(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         CREATE TABLE IF NOT EXISTS disk_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp REAL NOT NULL,
            read_bytes INTEGER NOT NULL,
            write_bytes INTEGER NOT NULL,
            read_speed INTEGER NOT NULL,
            write_speed INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS alltime_totals (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            read_bytes INTEGER NOT NULL DEFAULT 0,
            write_bytes INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS process_history (
            name TEXT PRIMARY KEY,
            read_bytes INTEGER NOT NULL DEFAULT 0,
            write_bytes INTEGER NOT NULL DEFAULT 0
         );
         INSERT OR IGNORE INTO alltime_totals (id, read_bytes, write_bytes) VALUES (1, 0, 0);",
    )
    .execute(&pool)
    .await?;

    // Recover previous session data if any
    // Get max values from disk_stats (previous session)
    let (read_total, write_total) = get_max_session_totals(&pool).await?;

    if read_total > 0 || write_total > 0 {
        println!("[DB] Recovering previous session data: Read={} bytes, Write={} bytes", read_total, write_total);
        match update_alltime_totals(&pool, read_total, write_total).await {
            Ok(_) => println!("[DB] Successfully updated all-time totals."),
            Err(e) => eprintln!("[DB] Failed to update all-time totals: {}", e),
        }
    } else {
        println!("[DB] No previous session data to recover.");
    }

    // Clear disk_stats for the new session
    sqlx::query("DELETE FROM disk_stats").execute(&pool).await?;

    Ok(pool)
}

pub async fn insert_stats_batch(
    pool: &Pool<Sqlite>,
    stats: &[DiskStat],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;

    for stat in stats {
        sqlx::query(
            "INSERT INTO disk_stats (timestamp, read_bytes, write_bytes, read_speed, write_speed)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(stat.timestamp)
        .bind(stat.read_bytes as i64)
        .bind(stat.write_bytes as i64)
        .bind(stat.read_speed as i64)
        .bind(stat.write_speed as i64)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;
    Ok(())
}

/// Gets the maximum session totals from disk_stats (current session totals before reset)
pub async fn get_max_session_totals(pool: &Pool<Sqlite>) -> Result<(u64, u64), sqlx::Error> {
    // Get the maximum values from the database (cumulative totals for current session)
    let result: (Option<i64>, Option<i64>) =
        sqlx::query_as("SELECT MAX(read_bytes), MAX(write_bytes) FROM disk_stats")
            .fetch_one(pool)
            .await?;

    let read_total = result.0.unwrap_or(0) as u64;
    let write_total = result.1.unwrap_or(0) as u64;

    Ok((read_total, write_total))
}

/// Updates the all-time totals by adding the session totals
pub async fn update_alltime_totals(
    pool: &Pool<Sqlite>,
    read_bytes_delta: u64,
    write_bytes_delta: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE alltime_totals SET 
            read_bytes = read_bytes + ?, 
            write_bytes = write_bytes + ? 
         WHERE id = 1",
    )
    .bind(read_bytes_delta as i64)
    .bind(write_bytes_delta as i64)
    .execute(pool)
    .await?;

    Ok(())
}

/// Gets the all-time total read and write bytes from the database
/// Returns (total_read_bytes, total_write_bytes)
pub async fn get_alltime_totals(pool: &Pool<Sqlite>) -> Result<(u64, u64), sqlx::Error> {
    // Get the cumulative totals from alltime_totals table
    let result: (i64, i64) =
        sqlx::query_as("SELECT read_bytes, write_bytes FROM alltime_totals WHERE id = 1")
            .fetch_one(pool)
            .await
            .unwrap_or((0, 0));

    Ok((result.0 as u64, result.1 as u64))
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
    clear_disk_stats(pool).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    
    // Reset all-time totals
    clear_alltime_totals(pool).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Run VACUUM to reclaim space
    println!("[DB] Running VACUUM to reclaim space...");
    sqlx::query("VACUUM").execute(pool).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Checkpoint WAL to ensure everything is written to the main file
    println!("[DB] Running WAL Checkpoint...");
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)").execute(pool).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

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

pub async fn clear_alltime_totals(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE alltime_totals SET read_bytes = 0, write_bytes = 0 WHERE id = 1")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM process_history").execute(pool).await?;
    Ok(())
}

pub async fn get_process_history(pool: &Pool<Sqlite>) -> Result<std::collections::HashMap<String, (u64, u64)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, i64, i64)>("SELECT name, read_bytes, write_bytes FROM process_history")
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
    let mut transaction = pool.begin().await?;

    for (name, (read, write)) in stats {
        sqlx::query(
            "INSERT INTO process_history (name, read_bytes, write_bytes) 
             VALUES (?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET
             read_bytes = read_bytes + excluded.read_bytes,
             write_bytes = write_bytes + excluded.write_bytes",
        )
        .bind(name)
        .bind(read as i64)
        .bind(write as i64)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;
    Ok(())
}

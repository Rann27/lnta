use rusqlite::Connection;
use std::path::Path;

use crate::error::AppError;

const MIGRATIONS: &[(u32, &str)] = &[(1, include_str!("migrations/001_initial_schema.sql"))];

/// Initialize SQLite connection settings (foreign keys, WAL mode).
pub fn setup_connection(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;",
    )
    .map_err(|e| AppError::DatabaseError {
        message: format!("Failed to set pragmas: {e}"),
    })?;
    Ok(())
}

/// Applies all pending numbered migrations in order.
pub fn migrate(conn: &mut Connection) -> Result<(), AppError> {
    setup_connection(conn)?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )
    .map_err(|e| AppError::DatabaseError {
        message: format!("Failed to create _migrations table: {e}"),
    })?;

    for &(version, sql) in MIGRATIONS {
        let is_applied: bool = conn
            .query_row(
                "SELECT 1 FROM _migrations WHERE version = ?1",
                [version],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !is_applied {
            let tx = conn.transaction().map_err(|e| AppError::DatabaseError {
                message: format!("Failed to start transaction for migration {version}: {e}"),
            })?;

            tx.execute_batch(sql)
                .map_err(|e| AppError::MigrationError {
                    version,
                    message: e.to_string(),
                })?;

            let now = chrono::Utc::now().to_rfc3339();
            tx.execute(
                "INSERT INTO _migrations (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![version, now],
            )
            .map_err(|e| AppError::MigrationError {
                version,
                message: format!("Failed to record migration {version}: {e}"),
            })?;

            tx.commit().map_err(|e| AppError::MigrationError {
                version,
                message: format!("Failed to commit migration {version}: {e}"),
            })?;
        }
    }

    Ok(())
}

/// Creates an in-memory database with all migrations applied, ideal for unit testing.
pub fn open_in_memory() -> Result<Connection, AppError> {
    let mut conn = Connection::open_in_memory().map_err(|e| AppError::DatabaseError {
        message: format!("Failed to open in-memory db: {e}"),
    })?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// Opens a database file at the given path with all migrations applied.
pub fn open_path<P: AsRef<Path>>(path: P) -> Result<Connection, AppError> {
    let mut conn = Connection::open(path).map_err(|e| AppError::DatabaseError {
        message: format!("Failed to open db: {e}"),
    })?;
    migrate(&mut conn)?;
    Ok(conn)
}

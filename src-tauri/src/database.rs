use rusqlite::Connection;

const SCHEMA_VERSION: i64 = 2;

pub fn initialize_database(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS vault (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            format_version INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            password_hash TEXT
        );

        PRAGMA user_version = 2;
        ",
    )?;

    Ok(())
}

pub fn schema_version(connection: &Connection) -> rusqlite::Result<i64> {
    connection.query_row("PRAGMA user_version", [], |row| row.get(0))
}

pub fn expected_schema_version() -> i64 {
    SCHEMA_VERSION
}

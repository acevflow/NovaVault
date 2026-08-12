use rusqlite::Connection;

const SCHEMA_VERSION: i64 = 3;

pub fn initialize_database(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS vault (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            format_version INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            password_hash TEXT,
            encryption_salt BLOB
        );

        CREATE TABLE IF NOT EXISTS notes (
            id TEXT PRIMARY KEY NOT NULL,
            title TEXT NOT NULL,
            content BLOB NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS passwords (
            id TEXT PRIMARY KEY NOT NULL,
            title TEXT NOT NULL,
            username TEXT,
            secret BLOB NOT NULL,
            notes BLOB,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS folders (
            id TEXT PRIMARY KEY NOT NULL,
            parent_id TEXT,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(parent_id) REFERENCES folders(id)
        );

        CREATE TABLE IF NOT EXISTS files (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            folder_id TEXT,
            object_path TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(folder_id) REFERENCES folders(id)
        );

        PRAGMA user_version = 3;
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

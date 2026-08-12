use std::fs;
use std::path::PathBuf;

use rusqlite::{Connection, params};
use serde::Serialize;
use tauri::{AppHandle, Manager};

const APP_DATABASE_FILE: &str = "novavault.db";
const SCHEMA_VERSION: i64 = 1;

#[derive(Serialize)]
pub struct AppVault {
    pub id: String,
    pub name: String,
    pub path: String,
}

fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to determine application data directory: {error}"))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("Failed to create application data directory: {error}"))?;

    Ok(app_data_dir.join(APP_DATABASE_FILE))
}

pub fn open_database(app: &AppHandle) -> Result<Connection, String> {
    let path = database_path(app)?;

    Connection::open(&path).map_err(|error| format!("Failed to open application database: {error}"))
}

pub fn initialize(app: &AppHandle) -> Result<(), String> {
    let connection = open_database(app)?;

    connection
        .execute_batch(&format!(
            "
            CREATE TABLE IF NOT EXISTS vaults (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                last_opened_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_vaults_last_opened
            ON vaults(last_opened_at);

            PRAGMA user_version = {SCHEMA_VERSION};
            "
        ))
        .map_err(|error| format!("Failed to initialize application database: {error}"))?;

    Ok(())
}

pub fn register_vault(app: &AppHandle, id: &str, name: &str, path: &str) -> Result<(), String> {
    let connection = open_database(app)?;

    connection
        .execute(
            "
            INSERT INTO vaults (
                id,
                name,
                path
            )
            VALUES (?1, ?2, ?3)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path
            ",
            params![id, name, path],
        )
        .map_err(|error| format!("Failed to register Vault: {error}"))?;

    Ok(())
}

pub fn mark_vault_opened(app: &AppHandle, id: &str) -> Result<(), String> {
    let connection = open_database(app)?;

    let now = chrono::Utc::now().to_rfc3339();

    connection
        .execute(
            "
            UPDATE vaults
            SET last_opened_at = ?1
            WHERE id = ?2
            ",
            params![now, id],
        )
        .map_err(|error| format!("Failed to update last opened Vault: {error}"))?;

    Ok(())
}

pub fn get_saved_vaults(app: &AppHandle) -> Result<Vec<AppVault>, String> {
    let connection = open_database(app)?;

    let mut statement = connection
        .prepare(
            "
            SELECT id, name, path
            FROM vaults
            ORDER BY
                CASE
                    WHEN last_opened_at IS NULL THEN 1
                    ELSE 0
                END,
                last_opened_at DESC
            ",
        )
        .map_err(|error| format!("Failed to prepare Vault query: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(AppVault {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
            })
        })
        .map_err(|error| format!("Failed to read saved Vaults: {error}"))?;

    let mut vaults = Vec::new();

    for row in rows {
        let vault = row.map_err(|error| format!("Failed to read saved Vault entry: {error}"))?;

        vaults.push(vault);
    }

    Ok(vaults)
}

pub fn remove_vault(app: &AppHandle, id: &str) -> Result<(), String> {
    let connection = open_database(app)?;

    connection
        .execute("DELETE FROM vaults WHERE id = ?1", params![id])
        .map_err(|error| format!("Failed to remove Vault from application database: {error}"))?;

    Ok(())
}

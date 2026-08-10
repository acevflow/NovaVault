use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::database::initialize_database;

pub struct CreatedVault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

pub fn create_vault(
    vault_name: String,
    storage_location: String,
) -> Result<CreatedVault, String> {
    let name = vault_name.trim();

    if name.is_empty() {
        return Err("Vault name is required".to_string());
    }

    let parent = Path::new(&storage_location);

    if !parent.exists() {
        return Err("The selected storage location does not exist".to_string());
    }

    if !parent.is_dir() {
        return Err("The selected storage location is not a folder".to_string());
    }

    let vault_path = parent.join(name);

    if vault_path.exists() {
        return Err("A folder with this Vault name already exists".to_string());
    }

    fs::create_dir(&vault_path)
        .map_err(|error| format!("Failed to create Vault folder: {error}"))?;

    let data_path = vault_path.join("data");
    let objects_path = data_path.join("objects");

    if let Err(error) = fs::create_dir(&data_path) {
        let _ = fs::remove_dir_all(&vault_path);
        return Err(format!("Failed to create data directory: {error}"));
    }

    if let Err(error) = fs::create_dir(&objects_path) {
        let _ = fs::remove_dir_all(&vault_path);
        return Err(format!("Failed to create objects directory: {error}"));
    }

    let database_path = vault_path.join("vault.db");

    let connection = match Connection::open(&database_path) {
        Ok(connection) => connection,
        Err(error) => {
            let _ = fs::remove_dir_all(&vault_path);
            return Err(format!("Failed to create Vault database: {error}"));
        }
    };

    if let Err(error) = initialize_database(&connection) {
        drop(connection);
        let _ = fs::remove_dir_all(&vault_path);

        return Err(format!("Failed to initialize Vault database: {error}"));
    }

    let vault_id = generate_id();

    let now = chrono::Utc::now().to_rfc3339();

    if let Err(error) = connection.execute(
        "
        INSERT INTO vault (
            id,
            name,
            format_version,
            created_at,
            updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5)
        ",
        (
            &vault_id,
            name,
            1_i64,
            &now,
            &now,
        ),
    ) {
        drop(connection);
        let _ = fs::remove_dir_all(&vault_path);

        return Err(format!("Failed to save Vault metadata: {error}"));
    }

    Ok(CreatedVault {
        id: vault_id,
        name: name.to_string(),
        path: vault_path,
    })
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
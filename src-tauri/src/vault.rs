use std::fs;
use std::path::{Path, PathBuf};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand::rngs::OsRng;
use rusqlite::Connection;

use crate::database::initialize_database;

pub struct CreatedVault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub password_protected: bool,
}

pub fn create_vault(
    vault_name: String,
    storage_location: String,
    password: Option<String>,
) -> Result<CreatedVault, String> {
    let name = vault_name.trim();

    if name.is_empty() {
        return Err("Vault name is required".to_string());
    }

    let password = password.filter(|password| !password.is_empty());

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

    let password_hash = match password {
        Some(password) if !password.is_empty() => {
            let salt = SaltString::generate(&mut OsRng);

            Some(
                Argon2::default()
                    .hash_password(password.as_bytes(), &salt)
                    .map_err(|error| format!("Failed to hash Vault password: {error}"))?
                    .to_string(),
            )
        }

        _ => None,
    };

    if let Err(error) = connection.execute(
        "
    INSERT INTO vault (
        id,
        name,
        format_version,
        created_at,
        updated_at,
        password_hash
    )
    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
    ",
        (&vault_id, name, 1_i64, &now, &now, &password_hash),
    ) {
        drop(connection);
        let _ = fs::remove_dir_all(&vault_path);

        return Err(format!("Failed to save Vault metadata: {error}"));
    }

    Ok(CreatedVault {
        id: vault_id,
        name: name.to_string(),
        path: vault_path,
        password_protected: password_hash.is_some(),
    })
}

pub fn open_vault(vault_path: String, password: Option<String>) -> Result<CreatedVault, String> {
    let vault_path = PathBuf::from(vault_path);

    if !vault_path.exists() {
        return Err("The selected Vault does not exist".to_string());
    }

    if !vault_path.is_dir() {
        return Err("The selected Vault is not a folder".to_string());
    }

    let data_path = vault_path.join("data");
    let objects_path = data_path.join("objects");
    let database_path = vault_path.join("vault.db");

    if !data_path.is_dir() {
        return Err("The selected folder is not a valid Vault".to_string());
    }

    if !objects_path.is_dir() {
        return Err("The selected folder is not a valid Vault".to_string());
    }

    if !database_path.is_file() {
        return Err("The selected folder does not contain a Vault database".to_string());
    }

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    let (id, name, format_version, password_hash): (String, String, i64, Option<String>) =
        connection
            .query_row(
                "
            SELECT
                id,
                name,
                format_version,
                password_hash
            FROM vault
            LIMIT 1
            ",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|error| format!("Failed to read Vault metadata: {error}"))?;

    if format_version != 1 {
        return Err(format!(
            "Unsupported Vault format version: {format_version}"
        ));
    }

    match &password_hash {
        Some(hash) => {
            let password =
                password.ok_or_else(|| "This Vault is protected by a password".to_string())?;

            verify_password(&password, &hash)?;
        }

        None => {
            if password.is_some() {
                return Err("This Vault does not use password protection".to_string());
            }
        }
    }

    Ok(CreatedVault {
        id,
        name,
        path: vault_path,
        password_protected: password_hash.is_some(),
    })
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| format!("Failed to protect Vault password: {error}"))
}

fn verify_password(password: &str, password_hash: &str) -> Result<(), String> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|error| format!("Failed to read Vault password protection: {error}"))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| "Incorrect Vault password".to_string())
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn verify_vault_password(vault_path: String, password: String) -> Result<bool, String> {
    let vault_path = PathBuf::from(vault_path);

    if !vault_path.exists() {
        return Err("The selected Vault does not exist".to_string());
    }

    if !vault_path.is_dir() {
        return Err("The selected Vault is not a folder".to_string());
    }

    let database_path = vault_path.join("vault.db");

    if !database_path.is_file() {
        return Err("The selected folder does not contain a Vault database".to_string());
    }

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    let password_hash: Option<String> = connection
        .query_row(
            "
            SELECT password_hash
            FROM vault
            LIMIT 1
            ",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("Failed to read Vault password information: {error}"))?;

    let Some(password_hash) = password_hash else {
        return Ok(true);
    };

    let parsed_hash = PasswordHash::new(&password_hash)
        .map_err(|error| format!("Invalid Vault password hash: {error}"))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn is_vault_password_protected(vault_path: String) -> Result<bool, String> {
    let vault_path = PathBuf::from(vault_path);

    if !vault_path.exists() {
        return Err("The selected Vault does not exist".to_string());
    }

    if !vault_path.is_dir() {
        return Err("The selected Vault is not a folder".to_string());
    }

    let database_path = vault_path.join("vault.db");

    if !database_path.is_file() {
        return Err("The selected folder does not contain a Vault database".to_string());
    }

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    let password_hash: Option<String> = connection
        .query_row(
            "
            SELECT password_hash
            FROM vault
            LIMIT 1
            ",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("Failed to read Vault password information: {error}"))?;

    Ok(password_hash.is_some())
}

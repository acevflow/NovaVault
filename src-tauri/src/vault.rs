use std::fs;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::{RngCore, rngs::OsRng};
use rusqlite::{Connection, params};
use serde::Serialize;
use sha2::Sha256;

use crate::database::{initialize_database, schema_version};

const ENCRYPTION_SALT_LEN: usize = 16;
const AES_NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const PBKDF2_ITERATIONS: u32 = 150_000;

#[derive(Clone)]
pub struct CreatedVault {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub password_protected: bool,
    pub encryption_key: [u8; KEY_LEN],
}

#[derive(Serialize)]
pub struct VaultFolder {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct VaultFile {
    pub id: String,
    pub name: String,
    pub folder_id: Option<String>,
    pub object_path: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct VaultNote {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct VaultPassword {
    pub id: String,
    pub title: String,
    pub username: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct VaultContents {
    pub folders: Vec<VaultFolder>,
    pub files: Vec<VaultFile>,
    pub notes: Vec<VaultNote>,
    pub passwords: Vec<VaultPassword>,
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

    let encryption_salt = create_encryption_salt();
    let encryption_key = derive_encryption_key(password.as_deref(), &encryption_salt);

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
            password_hash,
            encryption_salt
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ",
        params![
            &vault_id,
            name,
            1_i64,
            &now,
            &now,
            &password_hash,
            &encryption_salt,
        ],
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
        encryption_key,
    })
}

fn create_encryption_salt() -> Vec<u8> {
    let mut salt = vec![0u8; ENCRYPTION_SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

fn ensure_vault_schema(connection: &Connection) -> rusqlite::Result<()> {
    let current_version = schema_version(connection)?;

    if current_version < 2 {
        connection.execute_batch(
            "
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

            PRAGMA user_version = 2;
            ",
        )?;
    }

    if current_version < 3 {
        if !column_exists(connection, "vault", "encryption_salt")? {
            connection.execute("ALTER TABLE vault ADD COLUMN encryption_salt BLOB", [])?;
        }

        connection.execute_batch(
            "
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
    }

    Ok(())
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = statement.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn load_vault_metadata(
    vault_path: &PathBuf,
) -> Result<(String, String, i64, Option<String>, Vec<u8>), String> {
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

    ensure_vault_schema(&connection)
        .map_err(|error| format!("Failed to migrate Vault schema: {error}"))?;

    let (id, name, format_version, password_hash, encryption_salt): (
        String,
        String,
        i64,
        Option<String>,
        Option<Vec<u8>>,
    ) = connection
        .query_row(
            "
            SELECT
                id,
                name,
                format_version,
                password_hash,
                encryption_salt
            FROM vault
            LIMIT 1
            ",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|error| format!("Failed to read Vault metadata: {error}"))?;

    let encryption_salt = match encryption_salt {
        Some(salt) if salt.len() >= ENCRYPTION_SALT_LEN => salt,
        _ => {
            let salt = create_encryption_salt();
            connection
                .execute(
                    "UPDATE vault SET encryption_salt = ?1 WHERE id = ?2",
                    params![&salt, &id],
                )
                .map_err(|error| format!("Failed to persist Vault encryption salt: {error}"))?;
            salt
        }
    };

    Ok((id, name, format_version, password_hash, encryption_salt))
}

pub fn open_vault(vault_path: String, password: Option<String>) -> Result<CreatedVault, String> {
    let vault_path = PathBuf::from(vault_path);
    let (id, name, _format_version, password_hash, encryption_salt) =
        load_vault_metadata(&vault_path)?;

    match &password_hash {
        Some(hash) => {
            let password =
                password.ok_or_else(|| "This Vault is protected by a password".to_string())?;

            verify_password(&password, &hash)?;

            let encryption_key = derive_encryption_key(Some(&password), &encryption_salt);

            Ok(CreatedVault {
                id,
                name,
                path: vault_path,
                password_protected: true,
                encryption_key,
            })
        }
        None => {
            if password.is_some() {
                return Err("This Vault does not use password protection".to_string());
            }

            let encryption_key = derive_encryption_key(None, &encryption_salt);

            Ok(CreatedVault {
                id,
                name,
                path: vault_path,
                password_protected: false,
                encryption_key,
            })
        }
    }
}

pub fn open_vault_without_password(vault_path: String) -> Result<CreatedVault, String> {
    let vault_path = PathBuf::from(vault_path);
    let (id, name, _format_version, password_hash, encryption_salt) =
        load_vault_metadata(&vault_path)?;

    let encryption_key = derive_encryption_key(None, &encryption_salt);

    Ok(CreatedVault {
        id,
        name,
        path: vault_path,
        password_protected: password_hash.is_some(),
        encryption_key,
    })
}

fn derive_encryption_key(password: Option<&str>, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2::<Hmac<Sha256>>(
        password.unwrap_or("").as_bytes(),
        salt,
        PBKDF2_ITERATIONS,
        &mut key,
    );
    key
}

fn encrypt_bytes(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|error| format!("Failed to initialize encryption cipher: {error}"))?;

    let mut nonce_bytes = [0u8; AES_NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|error| format!("Failed to encrypt payload: {error}"))?;

    let mut output = nonce_bytes.to_vec();
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

fn decrypt_bytes(key: &[u8; KEY_LEN], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.len() < AES_NONCE_LEN {
        return Err("Invalid encrypted payload".to_string());
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|error| format!("Failed to initialize decryption cipher: {error}"))?;
    let (nonce_bytes, cipher_text) = ciphertext.split_at(AES_NONCE_LEN);

    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), cipher_text)
        .map_err(|error| format!("Failed to decrypt payload: {error}"))
}

fn encrypt_string(key: &[u8; KEY_LEN], value: &str) -> Result<Vec<u8>, String> {
    encrypt_bytes(key, value.as_bytes())
}

fn decrypt_string(key: &[u8; KEY_LEN], value: &[u8]) -> Result<String, String> {
    let bytes = decrypt_bytes(key, value)?;
    String::from_utf8(bytes).map_err(|error| format!("Failed to decode decrypted content: {error}"))
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

pub fn list_vault_contents(
    vault_path: String,
    encryption_key: &[u8; KEY_LEN],
) -> Result<VaultContents, String> {
    let vault_path = PathBuf::from(vault_path);
    let database_path = vault_path.join("vault.db");

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    ensure_vault_schema(&connection)
        .map_err(|error| format!("Failed to ensure Vault schema: {error}"))?;

    let mut folder_rows = connection
        .prepare(
            "SELECT id, parent_id, name, created_at, updated_at FROM folders ORDER BY name ASC",
        )
        .map_err(|error| format!("Failed to prepare folders query: {error}"))?;

    let folders = folder_rows
        .query_map([], |row| {
            Ok(VaultFolder {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|error| format!("Failed to read folders: {error}"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| format!("Failed to load folder rows: {error}"))?;

    let mut file_rows = connection
        .prepare(
            "SELECT id, name, folder_id, object_path, created_at, updated_at FROM files ORDER BY name ASC",
        )
        .map_err(|error| format!("Failed to prepare files query: {error}"))?;

    let files = file_rows
        .query_map([], |row| {
            Ok(VaultFile {
                id: row.get(0)?,
                name: row.get(1)?,
                folder_id: row.get(2)?,
                object_path: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|error| format!("Failed to read files: {error}"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| format!("Failed to load file rows: {error}"))?;

    let mut note_rows = connection
        .prepare(
            "SELECT id, title, content, created_at, updated_at FROM notes ORDER BY created_at DESC",
        )
        .map_err(|error| format!("Failed to prepare notes query: {error}"))?;

    let mut notes = Vec::new();
    let mut note_rows_iter = note_rows
        .query([])
        .map_err(|error| format!("Failed to execute notes query: {error}"))?;

    while let Some(row) = note_rows_iter
        .next()
        .map_err(|error| format!("Failed to read note row: {error}"))?
    {
        let encrypted_content: Vec<u8> = row
            .get(2)
            .map_err(|error| format!("Failed to read note content: {error}"))?;
        let content = decrypt_string(encryption_key, &encrypted_content)?;

        notes.push(VaultNote {
            id: row
                .get(0)
                .map_err(|error| format!("Failed to read note id: {error}"))?,
            title: row
                .get(1)
                .map_err(|error| format!("Failed to read note title: {error}"))?,
            content,
            created_at: row
                .get(3)
                .map_err(|error| format!("Failed to read note created_at: {error}"))?,
            updated_at: row
                .get(4)
                .map_err(|error| format!("Failed to read note updated_at: {error}"))?,
        });
    }

    let mut password_rows = connection
        .prepare(
            "SELECT id, title, username, notes, created_at, updated_at FROM passwords ORDER BY created_at DESC",
        )
        .map_err(|error| format!("Failed to prepare passwords query: {error}"))?;

    let mut passwords = Vec::new();
    let mut password_rows_iter = password_rows
        .query([])
        .map_err(|error| format!("Failed to execute passwords query: {error}"))?;

    while let Some(row) = password_rows_iter
        .next()
        .map_err(|error| format!("Failed to read password row: {error}"))?
    {
        let note_data: Option<Vec<u8>> = row
            .get(3)
            .map_err(|error| format!("Failed to read password notes: {error}"))?;
        let notes = match note_data {
            Some(data) => Some(decrypt_string(encryption_key, &data)?),
            None => None,
        };

        passwords.push(VaultPassword {
            id: row
                .get(0)
                .map_err(|error| format!("Failed to read password id: {error}"))?,
            title: row
                .get(1)
                .map_err(|error| format!("Failed to read password title: {error}"))?,
            username: row
                .get(2)
                .map_err(|error| format!("Failed to read password username: {error}"))?,
            notes,
            created_at: row
                .get(4)
                .map_err(|error| format!("Failed to read password created_at: {error}"))?,
            updated_at: row
                .get(5)
                .map_err(|error| format!("Failed to read password updated_at: {error}"))?,
        });
    }

    Ok(VaultContents {
        folders,
        files,
        notes,
        passwords,
    })
}

pub fn get_password_secret(
    vault_path: String,
    password_id: String,
    encryption_key: &[u8; KEY_LEN],
) -> Result<String, String> {
    let vault_path = PathBuf::from(vault_path);
    let database_path = vault_path.join("vault.db");

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    let secret_blob: Vec<u8> = connection
        .query_row(
            "SELECT secret FROM passwords WHERE id = ?1",
            params![password_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("Failed to load password secret: {error}"))?;

    decrypt_string(encryption_key, &secret_blob)
}

pub fn create_folder(
    vault_path: String,
    parent_id: Option<String>,
    name: String,
) -> Result<VaultFolder, String> {
    let vault_path = PathBuf::from(vault_path);
    let database_path = vault_path.join("vault.db");

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    ensure_vault_schema(&connection)
        .map_err(|error| format!("Failed to ensure Vault schema: {error}"))?;

    let folder_id = generate_id();
    let now = chrono::Utc::now().to_rfc3339();

    connection
        .execute(
            "INSERT INTO folders (id, parent_id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&folder_id, &parent_id, &name, &now, &now],
        )
        .map_err(|error| format!("Failed to create folder: {error}"))?;

    Ok(VaultFolder {
        id: folder_id,
        parent_id,
        name,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn move_file(
    vault_path: String,
    file_id: String,
    target_folder_id: Option<String>,
) -> Result<(), String> {
    let vault_path = PathBuf::from(vault_path);
    let database_path = vault_path.join("vault.db");

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    ensure_vault_schema(&connection)
        .map_err(|error| format!("Failed to ensure Vault schema: {error}"))?;

    connection
        .execute(
            "UPDATE files SET folder_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                &target_folder_id,
                &chrono::Utc::now().to_rfc3339(),
                &file_id
            ],
        )
        .map_err(|error| format!("Failed to move file: {error}"))?;

    Ok(())
}

pub fn add_note(
    vault_path: String,
    title: String,
    content: String,
    encryption_key: &[u8; KEY_LEN],
) -> Result<VaultNote, String> {
    let vault_path = PathBuf::from(vault_path);
    let database_path = vault_path.join("vault.db");

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    ensure_vault_schema(&connection)
        .map_err(|error| format!("Failed to ensure Vault schema: {error}"))?;

    let encrypted_content = encrypt_string(encryption_key, &content)?;
    let note_id = generate_id();
    let now = chrono::Utc::now().to_rfc3339();

    connection
        .execute(
            "INSERT INTO notes (id, title, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&note_id, &title, &encrypted_content, &now, &now],
        )
        .map_err(|error| format!("Failed to save note: {error}"))?;

    Ok(VaultNote {
        id: note_id,
        title,
        content,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn add_password(
    vault_path: String,
    title: String,
    username: Option<String>,
    secret: String,
    notes: Option<String>,
    encryption_key: &[u8; KEY_LEN],
) -> Result<VaultPassword, String> {
    let vault_path = PathBuf::from(vault_path);
    let database_path = vault_path.join("vault.db");

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    ensure_vault_schema(&connection)
        .map_err(|error| format!("Failed to ensure Vault schema: {error}"))?;

    let encrypted_secret = encrypt_string(encryption_key, &secret)?;
    let encrypted_notes = match notes.as_deref() {
        Some(text) if !text.is_empty() => Some(encrypt_string(encryption_key, text)?),
        _ => None,
    };

    let password_id = generate_id();
    let now = chrono::Utc::now().to_rfc3339();

    connection
        .execute(
            "INSERT INTO passwords (id, title, username, secret, notes, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![&password_id, &title, &username, &encrypted_secret, &encrypted_notes, &now, &now],
        )
        .map_err(|error| format!("Failed to save password: {error}"))?;

    Ok(VaultPassword {
        id: password_id,
        title,
        username,
        notes,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn add_file(
    vault_path: String,
    source_path: String,
    folder_id: Option<String>,
    encryption_key: &[u8; KEY_LEN],
) -> Result<VaultFile, String> {
    let vault_path = PathBuf::from(vault_path);
    let data_path = vault_path.join("data");
    let objects_path = data_path.join("objects");
    let database_path = vault_path.join("vault.db");

    if !Path::new(&source_path).is_file() {
        return Err("Selected file does not exist".to_string());
    }

    let file_bytes =
        fs::read(&source_path).map_err(|error| format!("Failed to read source file: {error}"))?;
    let encrypted_bytes = encrypt_bytes(encryption_key, &file_bytes)?;
    let object_name = format!("{}.bin", generate_id());
    let object_path = objects_path.join(&object_name);

    fs::write(&object_path, encrypted_bytes)
        .map_err(|error| format!("Failed to save encrypted file object: {error}"))?;

    let file_id = generate_id();
    let now = chrono::Utc::now().to_rfc3339();

    let connection = Connection::open(&database_path)
        .map_err(|error| format!("Failed to open Vault database: {error}"))?;

    ensure_vault_schema(&connection)
        .map_err(|error| format!("Failed to ensure Vault schema: {error}"))?;

    connection
        .execute(
            "INSERT INTO files (id, name, folder_id, object_path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![&file_id, Path::new(&source_path).file_name().and_then(|name| name.to_str()).unwrap_or("unknown"), &folder_id, &format!("objects/{object_name}"), &now, &now],
        )
        .map_err(|error| format!("Failed to save file metadata: {error}"))?;

    Ok(VaultFile {
        id: file_id,
        name: Path::new(&source_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string(),
        folder_id,
        object_path: format!("objects/{object_name}"),
        created_at: now.clone(),
        updated_at: now,
    })
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

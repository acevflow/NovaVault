use std::fs;
use std::path::Path;

mod database;
mod vault;
mod vault_state;

#[tauri::command]
fn validate_storage_location(storage_location: String) -> Result<(), String> {
    let path = Path::new(&storage_location);

    if !path.exists() {
        return Err("The selected folder does not exist".to_string());
    }

    if !path.is_dir() {
        return Err("The selected path is not a folder".to_string());
    }

    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("Failed to read the selected folder: {error}"))?;

    if entries.next().is_some() {
        return Err("The selected folder is not empty".to_string());
    }

    Ok(())
}

#[tauri::command]
fn create_vault(
    vault_name: String,
    storage_location: String,
    state: tauri::State<'_, vault_state::VaultState>,
) -> Result<String, String> {
    let created = vault::create_vault(vault_name, storage_location)?;

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Failed to access Vault state".to_string())?;

    *vault = Some(vault_state::OpenVault {
        id: created.id,
        name: created.name,
        path: created.path.clone(),
    });

    Ok(created.path.to_string_lossy().into_owned())
}

#[tauri::command]
fn get_open_vault(
    state: tauri::State<'_, vault_state::VaultState>,
) -> Result<Option<String>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "Failed to access Vault state".to_string())?;

    Ok(vault
        .as_ref()
        .map(|vault| vault.path.to_string_lossy().into_owned()))
}

use vault_state::VaultState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(VaultState {
            vault: std::sync::Mutex::new(None),
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            validate_storage_location,
            create_vault,
            get_open_vault
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
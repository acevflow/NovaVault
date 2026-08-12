use std::path::{Path, PathBuf};

use tauri::Manager;

mod app_database;
mod database;
mod vault;
mod vault_state;

#[tauri::command]
fn validate_storage_location(storage_location: String, vault_name: Option<String>) -> Result<(), String> {
    let path = Path::new(&storage_location);

    if !path.exists() {
        return Err("The selected folder does not exist".to_string());
    }

    if !path.is_dir() {
        return Err("The selected path is not a folder".to_string());
    }

    if let Some(vault_name) = vault_name {
        let vault_path = path.join(vault_name.trim());

        if vault_path.exists() {
            return Err("A folder with this Vault name already exists in the selected folder".to_string());
        }
    }

    Ok(())
}

#[tauri::command]
fn create_vault(
    app: tauri::AppHandle,
    vault_name: String,
    storage_location: String,
    password: Option<String>,
    state: tauri::State<'_, vault_state::VaultState>,
) -> Result<String, String> {
    let created = vault::create_vault(vault_name, storage_location, password)?;

    app_database::register_vault(
        &app,
        &created.id,
        &created.name,
        &created.path.to_string_lossy(),
    )?;

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
fn open_vault(
    app: tauri::AppHandle,
    vault_path: String,
    password: Option<String>,
    state: tauri::State<'_, vault_state::VaultState>,
) -> Result<(), String> {
    let opened = vault::open_vault(vault_path, password)?;

    app_database::register_vault(
        &app,
        &opened.id,
        &opened.name,
        &opened.path.to_string_lossy(),
    )?;

    {
        let mut pending_unlock = state
            .pending_unlock
            .lock()
            .map_err(|_| "Failed to access pending Vault state".to_string())?;

        *pending_unlock = None;
    }

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Failed to access Vault state".to_string())?;

    *vault = Some(vault_state::OpenVault {
        id: opened.id,
        name: opened.name,
        path: opened.path,
    });

    Ok(())
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

fn restore_last_vault(
    app: &tauri::AppHandle,
    state: &vault_state::VaultState,
) -> Result<(), String> {
    let saved_vaults = app_database::get_saved_vaults(app)?;

    for saved_vault in saved_vaults {
        let vault_path = saved_vault.path.clone();

        let password_protected = match vault::is_vault_password_protected(vault_path.clone()) {
            Ok(protected) => protected,
            Err(_) => {
                app_database::remove_vault(app, &saved_vault.id)?;
                continue;
            }
        };

        if password_protected {
            let mut pending_unlock = state
                .pending_unlock
                .lock()
                .map_err(|_| "Failed to access pending Vault state".to_string())?;

            *pending_unlock = Some(vault_state::PendingUnlock {
                id: saved_vault.id.clone(),
                name: saved_vault.name.clone(),
                path: PathBuf::from(vault_path.clone()),
            });

            return Ok(());
        }

        let opened = match vault::open_vault(vault_path.clone(), None) {
            Ok(vault) => vault,
            Err(_) => {
                app_database::remove_vault(app, &saved_vault.id)?;
                continue;
            }
        };

        {
            let mut vault_state = state
                .vault
                .lock()
                .map_err(|_| "Failed to access Vault state".to_string())?;

            *vault_state = Some(vault_state::OpenVault {
                id: opened.id.clone(),
                name: opened.name.clone(),
                path: opened.path.clone(),
            });
        }

        app_database::mark_vault_opened(app, &opened.id)?;

        return Ok(());
    }

    Ok(())
}

#[tauri::command]
fn verify_vault_password(vault_path: String, password: String) -> Result<bool, String> {
    vault::verify_vault_password(vault_path, password)
}

#[tauri::command]
fn is_vault_password_protected(vault_path: String) -> Result<bool, String> {
    vault::is_vault_password_protected(vault_path)
}

#[tauri::command]
fn get_pending_unlock(
    state: tauri::State<'_, vault_state::VaultState>,
) -> Result<Option<String>, String> {
    let pending_unlock = state
        .pending_unlock
        .lock()
        .map_err(|_| "Failed to access pending Vault state".to_string())?;

    Ok(pending_unlock
        .as_ref()
        .map(|vault| vault.path.to_string_lossy().into_owned()))
}

#[tauri::command]
fn get_saved_vaults(app: tauri::AppHandle) -> Result<Vec<app_database::AppVault>, String> {
    let saved_vaults = app_database::get_saved_vaults(&app)?;
    let mut valid_vaults = Vec::new();

    for vault in saved_vaults {
        let path = Path::new(&vault.path);

        if path.exists() {
            valid_vaults.push(vault);
            continue;
        }

        let _ = app_database::remove_vault(&app, &vault.id);
    }

    Ok(valid_vaults)
}

#[tauri::command]
fn clear_pending_unlock(
    state: tauri::State<'_, vault_state::VaultState>,
) -> Result<(), String> {
    let mut pending_unlock = state
        .pending_unlock
        .lock()
        .map_err(|_| "Failed to access pending Vault state".to_string())?;

    *pending_unlock = None;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(vault_state::VaultState {
            vault: std::sync::Mutex::new(None),
            pending_unlock: std::sync::Mutex::new(None),
        })
        .setup(|app| {
            app_database::initialize(app.handle())?;

            let state = app.state::<vault_state::VaultState>();

            restore_last_vault(app.handle(), &state)?;

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            validate_storage_location,
            create_vault,
            open_vault,
            get_open_vault,
            verify_vault_password,
            is_vault_password_protected,
            get_pending_unlock,
            get_saved_vaults,
            clear_pending_unlock
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

//! NovaVault desktop application entry point.
//!
//! This crate is the thin Tauri 2 shell around `novavault-core`. It owns the
//! window lifecycle, the vault session, the minimal set of Tauri commands
//! and the auto-lock triggers. All cryptography and vault logic live in the
//! core; the webview never receives key material.
//!
//! Modules:
//!
//! - `autolock`: locking on inactivity and OS events, and resetting the UI.
//! - `commands`: the Tauri command handlers (the whole IPC surface).
//! - `session`: the unlocked vault and the logic behind every command.
//! - `dto`: command inputs and outputs.
//! - `error`: the typed, secret-free errors returned to the UI.

// Hide the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autolock;
mod commands;
mod dto;
mod error;
mod session;

use std::process::ExitCode;
use std::time::Instant;

use novavault_core::KdfParams;
use tauri::Manager;

use crate::commands::AppState;
use crate::session::Session;

/// File name of the vault inside the app data directory.
///
/// The location is fixed by the shell: the UI never sends a path, so a
/// compromised webview cannot point the app at arbitrary files.
const VAULT_FILE_NAME: &str = "vault.nvault";

fn main() -> ExitCode {
    let result = tauri::Builder::default()
        .setup(|app| {
            let path = app.path().app_data_dir()?.join(VAULT_FILE_NAME);
            let session = Session::new(path, KdfParams::default(), Instant::now());
            app.manage(AppState::new(session));
            autolock::start_idle_timer(app.handle().clone());
            Ok(())
        })
        // Keep in sync with `COMMANDS` in build.rs, which generates the
        // permissions granted in capabilities/default.json.
        .invoke_handler(tauri::generate_handler![
            commands::create_vault,
            commands::unlock,
            commands::lock,
            commands::change_master_password,
            commands::regenerate_recovery_code,
            commands::list_items,
            commands::create_item,
            commands::update_item,
            commands::delete_item,
        ])
        .run(tauri::generate_context!());
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Startup and runtime errors from Tauri carry no vault secrets.
            eprintln!("NovaVault failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    /// The capability must grant exactly the commands declared in build.rs,
    /// so a new command is never callable by accident.
    #[test]
    fn capability_grants_exactly_the_declared_commands() {
        let build = include_str!("../build.rs");
        let start = build.find("const COMMANDS").unwrap();
        let end = start + build[start..].find("];").unwrap();
        let mut declared: Vec<String> = build[start..end]
            .split('"')
            .skip(1)
            .step_by(2)
            .map(|name| format!("allow-{}", name.replace('_', "-")))
            .collect();
        declared.sort();

        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
        let mut granted: Vec<String> = capability["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|permission| permission.as_str().unwrap().to_owned())
            .collect();
        granted.sort();

        assert_eq!(declared.len(), 9);
        assert_eq!(granted, declared);
    }
}

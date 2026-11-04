//! Build script: generates Tauri context (config, capabilities, embedded assets).
//!
//! It also declares the app's commands. Tauri then generates one
//! `allow-<command>` permission per command and rejects every call that the
//! window's capability does not grant, so the IPC surface is exactly what
//! `capabilities/default.json` lists.

/// Every command the shell registers. Keep in sync with
/// `generate_handler!` in `src/main.rs`; a test checks the capability.
const COMMANDS: &[&str] = &[
    "vault_status",
    "create_vault",
    "unlock",
    "lock",
    "change_master_password",
    "regenerate_recovery_code",
    "list_items",
    "create_item",
    "update_item",
    "delete_item",
];

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manifest = tauri_build::AppManifest::new().commands(COMMANDS);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))?;
    Ok(())
}

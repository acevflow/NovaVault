//! NovaVault desktop application entry point.
//!
//! This crate is the thin Tauri 2 shell. It owns the window lifecycle and, as
//! features land, the minimal set of Tauri commands and OS integrations. All
//! cryptography and vault logic live in `novavault-core`; the webview never
//! receives key material.

// Hide the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dto;
mod error;
mod session;

use std::process::ExitCode;

fn main() -> ExitCode {
    match tauri::Builder::default().run(tauri::generate_context!()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Startup errors carry no secrets at this stage (no vault is open).
            eprintln!("NovaVault failed to start: {error}");
            ExitCode::FAILURE
        }
    }
}

use std::fs;
use std::path::Path;

#[tauri::command]
fn validate_storage_location(storage_location: String) -> Result<(), String> {
    let path = Path::new(&storage_location);

    if !path.exists() {
        return Err("The selected folder does not exist".to_string());
    }

    if !path.is_dir() {
        return Err("The selected path is not a folder".to_string());
    }

    let mut entries = fs::read_dir(path).map_err(|error| format!("Failed to read the selected folder: {error}"))?;

    if entries.next().is_some() {
        return Err("The selected folder is not empty".to_string());
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            validate_storage_location
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

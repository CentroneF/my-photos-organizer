mod library;

#[tauri::command]
fn inspect_library_folder(
    request: library::InspectLibraryFolderRequest,
) -> Result<library::InspectLibraryFolderResult, library::SetupLibraryError> {
    library::inspect_library_folder(request)
}

#[tauri::command]
fn setup_library(
    app: tauri::AppHandle,
    request: library::SetupLibraryRequest,
) -> Result<library::SetupLibraryResult, library::SetupLibraryError> {
    use tauri::Manager;

    let result = library::setup_library(request)?;
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        library::SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not access app settings: {error}"),
        )
    })?;
    library::remember_library_path(&app_data_dir, &result.folder_path)?;
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            inspect_library_folder,
            setup_library
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod import_source;
mod library;
mod review;
mod search;

#[tauri::command]
fn start_review(
    request: import_source::SaveImportSourceRequest,
) -> Result<review::ReviewState, review::ReviewError> {
    review::start_review(&request.folder_path)
}

#[tauri::command]
fn current_review_state() -> Result<review::ReviewState, review::ReviewError> {
    review::current_review_state()
}

#[tauri::command]
fn next_review_item(app: tauri::AppHandle) -> Result<review::ReviewItem, review::ReviewError> {
    review::next_review_item(app)
}

#[tauri::command]
fn skip_review_item(
    request: review::DecideRequest,
) -> Result<review::DecisionResult, review::ReviewError> {
    review::skip_review_item(request)
}

#[tauri::command]
fn import_review_item(
    request: review::ImportRequest,
) -> Result<review::DecisionResult, review::ReviewError> {
    review::import_review_item(request)
}

#[tauri::command]
fn substitute_review_item(
    request: review::SubstituteRequest,
) -> Result<review::DecisionResult, review::ReviewError> {
    review::substitute_review_item(request)
}

#[tauri::command]
fn search_library(
    app: tauri::AppHandle,
    request: search::SearchLibraryRequest,
) -> Result<search::SearchLibraryResult, search::SearchError> {
    search::search_library(app, request)
}

#[tauri::command]
fn suggest_library_tags(
    request: search::TagSuggestionRequest,
) -> Result<search::TagSuggestionResult, search::SearchError> {
    search::suggest_library_tags(request)
}

#[tauri::command]
fn recent_library_tags() -> Result<search::RecentTagsResult, search::SearchError> {
    search::recent_library_tags()
}

#[tauri::command]
fn lock_library() {
    library::lock_library();
}

#[tauri::command]
fn open_library_folder() -> Result<(), library::SetupLibraryError> {
    let library_path = library::active_library_path()?;
    tauri_plugin_opener::open_path(library_path, None::<&str>).map_err(|error| {
        library::SetupLibraryError::new(
            "folder_open_failed",
            format!("Could not open the protected library folder: {error}"),
        )
    })
}

#[tauri::command]
fn clean_library(
    app: tauri::AppHandle,
    request: library::CleanLibraryRequest,
) -> Result<library::CleanLibraryResult, library::SetupLibraryError> {
    use tauri::Manager;

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        library::SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not access app settings: {error}"),
        )
    })?;
    library::clean_library(request, &app_data_dir)
}

#[tauri::command]
fn save_import_source(
    app: tauri::AppHandle,
    request: import_source::SaveImportSourceRequest,
) -> Result<import_source::ImportSourceResult, import_source::ImportSourceError> {
    use tauri::Manager;

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        import_source::ImportSourceError::new(
            "settings_unavailable",
            format!("Could not access app settings: {error}"),
        )
    })?;
    let managed_library_path = library::read_remembered_library_path(&app_data_dir)
        .map_err(|error| import_source::ImportSourceError::new(error.code, error.message))?;
    import_source::save_import_source(&app_data_dir, &managed_library_path, request)
}

#[tauri::command]
fn remembered_import_source(
    app: tauri::AppHandle,
) -> Result<import_source::ImportSourceResult, import_source::ImportSourceError> {
    use tauri::Manager;

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        import_source::ImportSourceError::new(
            "settings_unavailable",
            format!("Could not access app settings: {error}"),
        )
    })?;
    import_source::remembered_import_source(&app_data_dir)
}

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

    let mut result = library::setup_library(request)?;
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        library::SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not access app settings: {error}"),
        )
    })?;
    if let Err(error) = library::remember_library_path(&app_data_dir, &result.folder_path) {
        result.message = format!(
            "{} The library was created, but its location could not be remembered ({error_message}). Open the existing library manually next time.",
            result.message,
            error_message = error.message
        );
    }
    Ok(result)
}

#[tauri::command]
fn remembered_library(
    app: tauri::AppHandle,
) -> Result<library::RememberedLibraryResult, library::SetupLibraryError> {
    use tauri::Manager;

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        library::SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not access app settings: {error}"),
        )
    })?;
    library::remembered_library(&app_data_dir)
}

#[tauri::command]
fn unlock_library(
    app: tauri::AppHandle,
    request: library::UnlockLibraryRequest,
) -> Result<library::UnlockLibraryResult, library::SetupLibraryError> {
    use tauri::Manager;

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        library::SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not access app settings: {error}"),
        )
    })?;
    let folder_path = library::read_remembered_library_path(&app_data_dir)?;
    library::unlock_library(&folder_path, request)
}

#[tauri::command]
fn open_existing_library(
    app: tauri::AppHandle,
    request: library::OpenExistingLibraryRequest,
) -> Result<library::UnlockLibraryResult, library::SetupLibraryError> {
    use tauri::Manager;

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        library::SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not access app settings: {error}"),
        )
    })?;
    let result = library::unlock_library(&request.folder_path, request.unlock)?;
    library::remember_library_path(&app_data_dir, &result.folder_path)?;
    Ok(result)
}

#[tauri::command]
fn recovery_question(
    request: library::RecoveryQuestionRequest,
) -> Result<library::RecoveryQuestionResult, library::SetupLibraryError> {
    library::recovery_question(&request.folder_path)
}

#[tauri::command]
fn reset_library_password(
    app: tauri::AppHandle,
    request: library::ResetLibraryPasswordRequest,
) -> Result<library::UnlockLibraryResult, library::SetupLibraryError> {
    use tauri::Manager;

    let folder_path = request.folder_path.clone();
    let mut result = library::reset_library_password(&folder_path, request)?;
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        library::SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not access app settings: {error}"),
        )
    })?;
    if let Err(error) = library::remember_library_path(&app_data_dir, &folder_path) {
        result.message = format!(
            "{} The password was reset, but this library location could not be remembered ({error_message}). Open the existing library manually next time.",
            result.message,
            error_message = error.message
        );
    }
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .invoke_handler(tauri::generate_handler![
            inspect_library_folder,
            save_import_source,
            remembered_import_source,
            start_review,
            current_review_state,
            next_review_item,
            skip_review_item,
            import_review_item,
            substitute_review_item,
            search_library,
            suggest_library_tags,
            recent_library_tags,
            lock_library,
            open_library_folder,
            clean_library,
            setup_library,
            remembered_library,
            unlock_library,
            open_existing_library,
            recovery_question,
            reset_library_password
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::{
    fs,
    path::{Path, PathBuf},
};

use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};

const IMPORT_SOURCE_POINTER_FILE: &str = "selected-import-source.json";
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveImportSourceRequest {
    pub folder_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceResult {
    pub state: &'static str,
    pub folder_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceError {
    pub code: &'static str,
    pub message: String,
}

impl ImportSourceError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Deserialize)]
struct ImportSourcePointer {
    format_version: u32,
    folder_path: String,
}

pub fn save_import_source(
    app_data_dir: &Path,
    managed_library_path: &str,
    request: SaveImportSourceRequest,
) -> Result<ImportSourceResult, ImportSourceError> {
    let source = PathBuf::from(request.folder_path.trim());
    if request.folder_path.trim().is_empty() {
        return Err(ImportSourceError::new(
            "missing_folder",
            "Choose a folder to use as the import source.",
        ));
    }

    let source_metadata = fs::metadata(&source).map_err(|_| {
        ImportSourceError::new("folder_unavailable", "The selected folder is unavailable.")
    })?;
    if !source_metadata.is_dir() {
        return Err(ImportSourceError::new(
            "not_a_folder",
            "Select a folder, not a file.",
        ));
    }

    let canonical_source = source.canonicalize().map_err(|_| {
        ImportSourceError::new("folder_unavailable", "The selected folder is unavailable.")
    })?;
    let canonical_library = Path::new(managed_library_path)
        .canonicalize()
        .map_err(|_| {
            ImportSourceError::new(
                "library_unavailable",
                "The protected library is unavailable, so an import source cannot be selected yet.",
            )
        })?;
    if canonical_source == canonical_library {
        return Err(ImportSourceError::new(
            "source_is_library",
            "The import folder must be separate from your protected library. Nothing was changed.",
        ));
    }

    let folder_path = source.display().to_string();
    write_import_source_pointer(app_data_dir, &folder_path)?;
    Ok(ImportSourceResult {
        state: "ready",
        folder_path: Some(folder_path),
    })
}

pub fn remembered_import_source(
    app_data_dir: &Path,
) -> Result<ImportSourceResult, ImportSourceError> {
    let folder_path = match read_import_source_pointer(app_data_dir) {
        Ok(folder_path) => folder_path,
        Err(error) if error.code == "source_not_remembered" => {
            return Ok(ImportSourceResult {
                state: "missing",
                folder_path: None,
            });
        }
        Err(error) => return Err(error),
    };

    let is_available = fs::metadata(&folder_path)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false);
    Ok(ImportSourceResult {
        state: if is_available { "ready" } else { "stale" },
        folder_path: Some(folder_path),
    })
}

fn read_import_source_pointer(app_data_dir: &Path) -> Result<String, ImportSourceError> {
    let bytes = fs::read(app_data_dir.join(IMPORT_SOURCE_POINTER_FILE)).map_err(|_| {
        ImportSourceError::new(
            "source_not_remembered",
            "No import folder has been selected.",
        )
    })?;
    let pointer: ImportSourcePointer = serde_json::from_slice(&bytes).map_err(|_| {
        ImportSourceError::new(
            "source_pointer_invalid",
            "The remembered import folder location is invalid.",
        )
    })?;
    if pointer.format_version != FORMAT_VERSION || pointer.folder_path.trim().is_empty() {
        return Err(ImportSourceError::new(
            "source_pointer_invalid",
            "The remembered import folder location is invalid.",
        ));
    }
    Ok(pointer.folder_path)
}

fn write_import_source_pointer(
    app_data_dir: &Path,
    folder_path: &str,
) -> Result<(), ImportSourceError> {
    fs::create_dir_all(app_data_dir).map_err(|error| {
        ImportSourceError::new(
            "settings_unavailable",
            format!("Could not save the import folder: {error}"),
        )
    })?;
    let pointer =
        serde_json::json!({ "format_version": FORMAT_VERSION, "folder_path": folder_path });
    let temporary_pointer = app_data_dir.join(format!(
        "{IMPORT_SOURCE_POINTER_FILE}.pending-{}",
        random_suffix()
    ));
    fs::write(&temporary_pointer, pointer.to_string()).map_err(|error| {
        ImportSourceError::new(
            "settings_unavailable",
            format!("Could not save the import folder: {error}"),
        )
    })?;
    fs::rename(
        &temporary_pointer,
        app_data_dir.join(IMPORT_SOURCE_POINTER_FILE),
    )
    .map_err(|error| {
        let _ = fs::remove_file(&temporary_pointer);
        ImportSourceError::new(
            "settings_unavailable",
            format!("Could not save the import folder: {error}"),
        )
    })
}

fn random_suffix() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn save(
        settings: &Path,
        library: &Path,
        source: &Path,
    ) -> Result<ImportSourceResult, ImportSourceError> {
        save_import_source(
            settings,
            &library.display().to_string(),
            SaveImportSourceRequest {
                folder_path: source.display().to_string(),
            },
        )
    }

    #[test]
    fn saves_and_reloads_a_source_without_touching_its_contents() {
        let settings = tempdir().unwrap();
        let library = tempdir().unwrap();
        let source = tempdir().unwrap();
        let photo = source.path().join("family.jpg");
        fs::write(&photo, b"original photo bytes").unwrap();

        let saved = save(settings.path(), library.path(), source.path()).unwrap();
        let reloaded = remembered_import_source(settings.path()).unwrap();

        assert_eq!(saved.state, "ready");
        assert_eq!(reloaded.state, "ready");
        assert_eq!(reloaded.folder_path, saved.folder_path);
        assert_eq!(fs::read(photo).unwrap(), b"original photo bytes");
        assert_eq!(fs::read_dir(source.path()).unwrap().count(), 1);
    }

    #[test]
    fn returns_missing_when_no_source_has_been_selected() {
        let settings = tempdir().unwrap();
        let result = remembered_import_source(settings.path()).unwrap();
        assert_eq!(result.state, "missing");
        assert_eq!(result.folder_path, None);
    }

    #[test]
    fn retains_a_remembered_source_when_it_becomes_unavailable() {
        let settings = tempdir().unwrap();
        let library = tempdir().unwrap();
        let source = tempdir().unwrap();
        let selected_path = source.path().display().to_string();
        save(settings.path(), library.path(), source.path()).unwrap();
        source.close().unwrap();

        let result = remembered_import_source(settings.path()).unwrap();
        assert_eq!(result.state, "stale");
        assert_eq!(result.folder_path.as_deref(), Some(selected_path.as_str()));
    }

    #[test]
    fn rejects_invalid_selections_without_replacing_the_current_pointer() {
        let settings = tempdir().unwrap();
        let library = tempdir().unwrap();
        let source = tempdir().unwrap();
        let retained = tempdir().unwrap();
        let source_file = source.path().join("original.mov");
        fs::write(&source_file, b"do not modify").unwrap();
        save(settings.path(), library.path(), retained.path()).unwrap();

        let error = save(settings.path(), library.path(), &source_file).unwrap_err();

        assert_eq!(error.code, "not_a_folder");
        assert_eq!(fs::read(&source_file).unwrap(), b"do not modify");
        assert_eq!(
            remembered_import_source(settings.path())
                .unwrap()
                .folder_path
                .as_deref(),
            Some(retained.path().display().to_string().as_str())
        );
    }

    #[test]
    fn rejects_the_managed_library_without_replacing_the_current_pointer() {
        let settings = tempdir().unwrap();
        let library = tempdir().unwrap();
        let retained = tempdir().unwrap();
        save(settings.path(), library.path(), retained.path()).unwrap();

        let error = save(settings.path(), library.path(), library.path()).unwrap_err();

        assert_eq!(error.code, "source_is_library");
        assert_eq!(
            remembered_import_source(settings.path())
                .unwrap()
                .folder_path
                .as_deref(),
            Some(retained.path().display().to_string().as_str())
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlink_alias_of_the_managed_library() {
        use std::os::unix::fs::symlink;

        let settings = tempdir().unwrap();
        let library = tempdir().unwrap();
        let alias_parent = tempdir().unwrap();
        let alias = alias_parent.path().join("library-alias");
        symlink(library.path(), &alias).unwrap();

        let error = save(settings.path(), library.path(), &alias).unwrap_err();

        assert_eq!(error.code, "source_is_library");
    }
}

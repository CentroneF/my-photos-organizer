use std::{
    fs,
    path::{Path, PathBuf},
};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::{rngs::OsRng, RngCore};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const STATE_DIR: &str = ".photo-handler";
const MARKER_FILE: &str = "library.json";
const DATABASE_FILE: &str = "catalogue.db";
const LIBRARY_POINTER_FILE: &str = "selected-library.json";
const FORMAT_VERSION: u32 = 1;
const KEY_BYTES: usize = 32;
const SALT_BYTES: usize = 16;
const NONCE_BYTES: usize = 12;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupLibraryRequest {
    pub folder_path: String,
    pub password: String,
    pub password_confirmation: String,
    pub recovery_question: String,
    pub recovery_answer: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupLibraryResult {
    pub folder_path: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectLibraryFolderRequest {
    pub folder_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectLibraryFolderResult {
    pub folder_path: String,
    pub state: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupLibraryError {
    pub code: &'static str,
    pub message: String,
}

impl SetupLibraryError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct LibraryMarker {
    format_version: u32,
    password_wrap: KeyWrap,
    recovery_wrap: KeyWrap,
    recovery_question: String,
}

#[derive(Serialize, Deserialize)]
struct KeyWrap {
    salt_hex: String,
    nonce_hex: String,
    ciphertext_hex: String,
}

pub fn setup_library(
    request: SetupLibraryRequest,
) -> Result<SetupLibraryResult, SetupLibraryError> {
    validate_request(&request)?;
    let folder = PathBuf::from(&request.folder_path);
    validate_empty_writable_folder(&folder)?;

    let state_dir = folder.join(STATE_DIR);
    // State is created only after the selected folder has passed every non-mutating check.
    fs::create_dir(&state_dir).map_err(|error| {
        SetupLibraryError::new(
            "folder_not_writable",
            format!("Photo Handler could not create its state folder: {error}"),
        )
    })?;

    let initialization = (|| -> Result<(), SetupLibraryError> {
        let mut database_key = [0_u8; KEY_BYTES];
        OsRng.fill_bytes(&mut database_key);
        let marker = LibraryMarker {
            format_version: FORMAT_VERSION,
            password_wrap: wrap_key(&database_key, &request.password)?,
            recovery_wrap: wrap_key(&database_key, &request.recovery_answer)?,
            recovery_question: request.recovery_question.trim().to_owned(),
        };
        initialize_catalogue(&state_dir.join(DATABASE_FILE), &database_key)?;
        let marker_json = serde_json::to_vec_pretty(&marker)
            .map_err(|error| SetupLibraryError::new("initialization_failed", error.to_string()))?;
        fs::write(state_dir.join(MARKER_FILE), marker_json).map_err(|error| {
            SetupLibraryError::new(
                "initialization_failed",
                format!("Could not write library marker: {error}"),
            )
        })?;
        database_key.fill(0);
        Ok(())
    })();

    if let Err(error) = initialization {
        // This directory was created by this invocation, so cleanup never affects user content.
        let _ = fs::remove_dir_all(&state_dir);
        return Err(error);
    }

    Ok(SetupLibraryResult {
        folder_path: folder.display().to_string(),
        message: "Protected library created. Only Photo Handler state was added; no media was copied, encrypted, moved, or modified.".to_owned(),
    })
}

pub fn inspect_library_folder(
    request: InspectLibraryFolderRequest,
) -> Result<InspectLibraryFolderResult, SetupLibraryError> {
    let folder = PathBuf::from(request.folder_path.trim());
    let metadata = fs::metadata(&folder).map_err(|_| {
        SetupLibraryError::new("folder_unavailable", "The selected folder is unavailable.")
    })?;
    if !metadata.is_dir() {
        return Err(SetupLibraryError::new(
            "not_a_folder",
            "Select a folder, not a file.",
        ));
    }

    let mut entries = fs::read_dir(&folder).map_err(|_| {
        SetupLibraryError::new(
            "folder_unavailable",
            "The selected folder cannot be inspected.",
        )
    })?;
    if entries.next().is_none() {
        return Ok(InspectLibraryFolderResult {
            folder_path: folder.display().to_string(),
            state: "new",
        });
    }

    let state_dir = folder.join(STATE_DIR);
    if state_dir.join(MARKER_FILE).is_file() && state_dir.join(DATABASE_FILE).is_file() {
        return Ok(InspectLibraryFolderResult {
            folder_path: folder.display().to_string(),
            state: "existing",
        });
    }

    Err(SetupLibraryError::new(
        "folder_not_empty",
        "This folder is not empty and is not a recognized Photo Handler library. Nothing was changed.",
    ))
}

pub fn remember_library_path(app_data_dir: &Path, folder: &str) -> Result<(), SetupLibraryError> {
    fs::create_dir_all(app_data_dir).map_err(|error| {
        SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not save the selected library: {error}"),
        )
    })?;
    let pointer = serde_json::json!({ "format_version": FORMAT_VERSION, "folder_path": folder });
    fs::write(app_data_dir.join(LIBRARY_POINTER_FILE), pointer.to_string()).map_err(|error| {
        SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not save the selected library: {error}"),
        )
    })
}

fn validate_request(request: &SetupLibraryRequest) -> Result<(), SetupLibraryError> {
    if request.folder_path.trim().is_empty() {
        return Err(SetupLibraryError::new(
            "missing_folder",
            "Choose an empty folder for the managed library.",
        ));
    }
    if request.password.is_empty() {
        return Err(SetupLibraryError::new(
            "missing_password",
            "Enter a password.",
        ));
    }
    if request.password != request.password_confirmation {
        return Err(SetupLibraryError::new(
            "password_mismatch",
            "The password confirmation does not match.",
        ));
    }
    if request.recovery_question.trim().is_empty() {
        return Err(SetupLibraryError::new(
            "missing_recovery_question",
            "Enter a recovery question.",
        ));
    }
    if request.recovery_answer.trim().is_empty() {
        return Err(SetupLibraryError::new(
            "missing_recovery_answer",
            "Enter a recovery answer.",
        ));
    }
    Ok(())
}

fn validate_empty_writable_folder(folder: &Path) -> Result<(), SetupLibraryError> {
    let metadata = fs::metadata(folder).map_err(|_| {
        SetupLibraryError::new("folder_unavailable", "The selected folder is unavailable.")
    })?;
    if !metadata.is_dir() {
        return Err(SetupLibraryError::new(
            "not_a_folder",
            "Select a folder, not a file.",
        ));
    }
    let mut entries = fs::read_dir(folder).map_err(|_| {
        SetupLibraryError::new(
            "folder_not_writable",
            "The selected folder cannot be read or written.",
        )
    })?;
    if entries.next().is_some() {
        return Err(SetupLibraryError::new(
            "folder_not_empty",
            "Choose an empty folder. Existing files and folders are never changed.",
        ));
    }
    Ok(())
}

fn wrap_key(database_key: &[u8; KEY_BYTES], secret: &str) -> Result<KeyWrap, SetupLibraryError> {
    let mut salt = [0_u8; SALT_BYTES];
    let mut nonce = [0_u8; NONCE_BYTES];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let mut derived_key = derive_key(secret, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&derived_key).map_err(|_| {
        SetupLibraryError::new(
            "initialization_failed",
            "Could not protect the catalogue key.",
        )
    })?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), database_key.as_ref())
        .map_err(|_| {
            SetupLibraryError::new(
                "initialization_failed",
                "Could not protect the catalogue key.",
            )
        })?;
    derived_key.fill(0);
    Ok(KeyWrap {
        salt_hex: hex::encode(salt),
        nonce_hex: hex::encode(nonce),
        ciphertext_hex: hex::encode(ciphertext),
    })
}

fn derive_key(secret: &str, salt: &[u8; SALT_BYTES]) -> Result<[u8; KEY_BYTES], SetupLibraryError> {
    let params = Params::new(19 * 1024, 2, 1, Some(KEY_BYTES)).map_err(|_| {
        SetupLibraryError::new(
            "initialization_failed",
            "Invalid key-derivation parameters.",
        )
    })?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0_u8; KEY_BYTES];
    argon2
        .hash_password_into(secret.as_bytes(), salt, &mut key)
        .map_err(|_| {
            SetupLibraryError::new(
                "initialization_failed",
                "Could not derive a protection key.",
            )
        })?;
    Ok(key)
}

fn initialize_catalogue(
    path: &Path,
    database_key: &[u8; KEY_BYTES],
) -> Result<(), SetupLibraryError> {
    let connection = Connection::open(path).map_err(|error| {
        SetupLibraryError::new(
            "initialization_failed",
            format!("Could not create catalogue: {error}"),
        )
    })?;
    connection
        .pragma_update(None, "key", format!("x'{}'", hex::encode(database_key)))
        .map_err(|error| {
            SetupLibraryError::new(
                "initialization_failed",
                format!("Could not encrypt catalogue: {error}"),
            )
        })?;
    connection.execute_batch("BEGIN IMMEDIATE; CREATE TABLE schema_migrations (version INTEGER NOT NULL); INSERT INTO schema_migrations (version) VALUES (1); CREATE TABLE library_identity (id INTEGER PRIMARY KEY CHECK (id = 1), format_version INTEGER NOT NULL); INSERT INTO library_identity (id, format_version) VALUES (1, 1); COMMIT;")
        .map_err(|error| SetupLibraryError::new("initialization_failed", format!("Could not initialize catalogue schema: {error}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn request(path: &Path) -> SetupLibraryRequest {
        SetupLibraryRequest {
            folder_path: path.display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "First pet?".into(),
            recovery_answer: "Mochi".into(),
        }
    }

    #[test]
    fn initializes_only_its_own_state_in_an_empty_folder() {
        let directory = tempdir().unwrap();
        let result = setup_library(request(directory.path())).unwrap();
        assert_eq!(result.folder_path, directory.path().display().to_string());
        let state = directory.path().join(STATE_DIR);
        assert!(state.join(MARKER_FILE).is_file());
        assert!(state.join(DATABASE_FILE).is_file());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn inspection_distinguishes_empty_and_existing_libraries() {
        let directory = tempdir().unwrap();
        let empty = inspect_library_folder(InspectLibraryFolderRequest {
            folder_path: directory.path().display().to_string(),
        })
        .unwrap();
        assert_eq!(empty.state, "new");

        setup_library(request(directory.path())).unwrap();
        let existing = inspect_library_folder(InspectLibraryFolderRequest {
            folder_path: directory.path().display().to_string(),
        })
        .unwrap();
        assert_eq!(existing.state, "existing");
    }

    #[test]
    fn inspection_rejects_foreign_content_without_mutation() {
        let directory = tempdir().unwrap();
        let original = directory.path().join("family.png");
        fs::write(&original, b"original bytes").unwrap();
        let error = inspect_library_folder(InspectLibraryFolderRequest {
            folder_path: directory.path().display().to_string(),
        })
        .unwrap_err();
        assert_eq!(error.code, "folder_not_empty");
        assert_eq!(fs::read(original).unwrap(), b"original bytes");
    }

    #[test]
    fn rejects_non_empty_folder_without_changing_its_file() {
        let directory = tempdir().unwrap();
        let original = directory.path().join("original.jpg");
        fs::write(&original, b"do not touch").unwrap();
        let error = setup_library(request(directory.path())).unwrap_err();
        assert_eq!(error.code, "folder_not_empty");
        assert_eq!(fs::read(&original).unwrap(), b"do not touch");
        assert!(!directory.path().join(STATE_DIR).exists());
    }

    #[test]
    fn duplicate_initialization_is_rejected_without_overwrite() {
        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();
        let marker = fs::read(directory.path().join(STATE_DIR).join(MARKER_FILE)).unwrap();
        let error = setup_library(request(directory.path())).unwrap_err();
        assert_eq!(error.code, "folder_not_empty");
        assert_eq!(
            fs::read(directory.path().join(STATE_DIR).join(MARKER_FILE)).unwrap(),
            marker
        );
    }

    #[test]
    fn catalogue_cannot_be_read_without_its_database_key() {
        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();
        let database = directory.path().join(STATE_DIR).join(DATABASE_FILE);
        let connection = Connection::open(database).unwrap();
        assert!(connection
            .query_row("SELECT count(*) FROM schema_migrations", [], |row| row
                .get::<_, i64>(0))
            .is_err());
    }

    #[test]
    fn remembers_only_the_selected_library_path_in_app_settings() {
        let settings = tempdir().unwrap();
        remember_library_path(settings.path(), "/example/library").unwrap();
        let pointer: serde_json::Value =
            serde_json::from_slice(&fs::read(settings.path().join(LIBRARY_POINTER_FILE)).unwrap())
                .unwrap();
        assert_eq!(pointer["folder_path"], "/example/library");
    }

    #[test]
    fn invalid_request_leaves_target_untouched() {
        let directory = tempdir().unwrap();
        let mut invalid = request(directory.path());
        invalid.password_confirmation = "different".into();
        assert_eq!(
            setup_library(invalid).unwrap_err().code,
            "password_mismatch"
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockLibraryRequest {
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenExistingLibraryRequest {
    pub folder_path: String,
    #[serde(flatten)]
    pub unlock: UnlockLibraryRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetLibraryPasswordRequest {
    pub folder_path: String,
    pub recovery_answer: String,
    pub new_password: String,
    pub new_password_confirmation: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryQuestionRequest {
    pub folder_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RememberedLibraryResult {
    pub state: &'static str,
    pub folder_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockLibraryResult {
    pub folder_path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryQuestionResult {
    pub question: String,
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

#[derive(Deserialize)]
struct LibraryPointer {
    format_version: u32,
    folder_path: String,
}

pub fn setup_library(
    request: SetupLibraryRequest,
) -> Result<SetupLibraryResult, SetupLibraryError> {
    validate_request(&request)?;
    let folder = PathBuf::from(&request.folder_path);
    validate_empty_writable_folder(&folder)?;

    let state_dir = folder.join(STATE_DIR);
    let temporary_state_dir = folder.join(format!("{STATE_DIR}.pending-{}", random_suffix()));
    // State is created only after the selected folder has passed every non-mutating check.
    fs::create_dir(&temporary_state_dir).map_err(|error| {
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
        initialize_catalogue(&temporary_state_dir.join(DATABASE_FILE), &database_key)?;
        let marker_json = serde_json::to_vec_pretty(&marker)
            .map_err(|error| SetupLibraryError::new("initialization_failed", error.to_string()))?;
        fs::write(temporary_state_dir.join(MARKER_FILE), marker_json).map_err(|error| {
            SetupLibraryError::new(
                "initialization_failed",
                format!("Could not write library marker: {error}"),
            )
        })?;
        database_key.fill(0);
        fs::rename(&temporary_state_dir, &state_dir).map_err(|error| {
            SetupLibraryError::new(
                "initialization_failed",
                format!("Could not publish protected library state: {error}"),
            )
        })?;
        Ok(())
    })();

    if let Err(error) = initialization {
        // This directory was created by this invocation, so cleanup never affects user content.
        let _ = fs::remove_dir_all(&temporary_state_dir);
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
    let temporary_pointer = app_data_dir.join(format!(
        "{LIBRARY_POINTER_FILE}.pending-{}",
        random_suffix()
    ));
    fs::write(&temporary_pointer, pointer.to_string()).map_err(|error| {
        SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not save the selected library: {error}"),
        )
    })?;
    fs::rename(&temporary_pointer, app_data_dir.join(LIBRARY_POINTER_FILE)).map_err(|error| {
        let _ = fs::remove_file(&temporary_pointer);
        SetupLibraryError::new(
            "settings_unavailable",
            format!("Could not save the selected library: {error}"),
        )
    })
}

fn random_suffix() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn read_remembered_library_path(app_data_dir: &Path) -> Result<String, SetupLibraryError> {
    let bytes = fs::read(app_data_dir.join(LIBRARY_POINTER_FILE)).map_err(|_| {
        SetupLibraryError::new(
            "library_not_remembered",
            "No protected library has been selected.",
        )
    })?;
    let pointer: LibraryPointer = serde_json::from_slice(&bytes).map_err(|_| {
        SetupLibraryError::new(
            "library_pointer_invalid",
            "The remembered library location is invalid.",
        )
    })?;
    if pointer.format_version != FORMAT_VERSION || pointer.folder_path.trim().is_empty() {
        return Err(SetupLibraryError::new(
            "library_pointer_invalid",
            "The remembered library location is invalid.",
        ));
    }
    Ok(pointer.folder_path)
}

pub fn remembered_library(
    app_data_dir: &Path,
) -> Result<RememberedLibraryResult, SetupLibraryError> {
    match read_remembered_library_path(app_data_dir) {
        Ok(folder_path) => match validate_existing_library(Path::new(&folder_path)) {
            Ok(_) => Ok(RememberedLibraryResult {
                state: "ready",
                folder_path: Some(folder_path),
            }),
            Err(_) => Ok(RememberedLibraryResult {
                state: "stale",
                folder_path: Some(folder_path),
            }),
        },
        Err(error) if error.code == "library_not_remembered" => Ok(RememberedLibraryResult {
            state: "missing",
            folder_path: None,
        }),
        Err(_) => Ok(RememberedLibraryResult {
            state: "stale",
            folder_path: None,
        }),
    }
}

pub fn unlock_library(
    folder_path: &str,
    request: UnlockLibraryRequest,
) -> Result<UnlockLibraryResult, SetupLibraryError> {
    if request.password.is_empty() {
        return Err(SetupLibraryError::new(
            "missing_password",
            "Enter your library password.",
        ));
    }
    let folder = PathBuf::from(folder_path.trim());
    let marker = validate_existing_library(&folder)?;
    let mut database_key = unwrap_key(&marker.password_wrap, &request.password)?;
    let database_result =
        validate_catalogue(&folder.join(STATE_DIR).join(DATABASE_FILE), &database_key);
    database_key.fill(0);
    database_result?;
    Ok(UnlockLibraryResult {
        folder_path: folder.display().to_string(),
        message: "Protected library unlocked. Your media files remain unchanged.".to_owned(),
    })
}

pub fn recovery_question(folder_path: &str) -> Result<RecoveryQuestionResult, SetupLibraryError> {
    let marker = validate_existing_library(Path::new(folder_path.trim()))?;
    Ok(RecoveryQuestionResult {
        question: marker.recovery_question,
    })
}

pub fn reset_library_password(
    folder_path: &str,
    request: ResetLibraryPasswordRequest,
) -> Result<UnlockLibraryResult, SetupLibraryError> {
    if request.recovery_answer.trim().is_empty() {
        return Err(SetupLibraryError::new(
            "missing_recovery_answer",
            "Enter the recovery answer.",
        ));
    }
    if request.new_password.is_empty() {
        return Err(SetupLibraryError::new(
            "missing_password",
            "Enter a new password.",
        ));
    }
    if request.new_password != request.new_password_confirmation {
        return Err(SetupLibraryError::new(
            "password_mismatch",
            "The password confirmation does not match.",
        ));
    }
    let folder = PathBuf::from(folder_path.trim());
    let mut marker = validate_existing_library(&folder)?;
    let mut database_key = unwrap_key(&marker.recovery_wrap, &request.recovery_answer)?;
    validate_catalogue(&folder.join(STATE_DIR).join(DATABASE_FILE), &database_key)?;
    let password_wrap = wrap_key(&database_key, &request.new_password)?;
    database_key.fill(0);
    marker.password_wrap = password_wrap;
    write_marker(&folder.join(STATE_DIR), &marker)?;
    Ok(UnlockLibraryResult {
        folder_path: folder.display().to_string(),
        message:
            "Password reset and protected library unlocked. Your media files remain unchanged."
                .to_owned(),
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

fn unwrap_key(wrap: &KeyWrap, secret: &str) -> Result<[u8; KEY_BYTES], SetupLibraryError> {
    let salt = decode_fixed::<SALT_BYTES>(&wrap.salt_hex)?;
    let nonce = decode_fixed::<NONCE_BYTES>(&wrap.nonce_hex)?;
    let ciphertext = hex::decode(&wrap.ciphertext_hex).map_err(|_| incorrect_credential_error())?;
    let mut derived_key = derive_key(secret, &salt).map_err(|_| incorrect_credential_error())?;
    let cipher =
        Aes256Gcm::new_from_slice(&derived_key).map_err(|_| incorrect_credential_error())?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| incorrect_credential_error())?;
    derived_key.fill(0);
    plaintext
        .try_into()
        .map_err(|_| incorrect_credential_error())
}

fn decode_fixed<const N: usize>(value: &str) -> Result<[u8; N], SetupLibraryError> {
    let bytes = hex::decode(value).map_err(|_| incorrect_credential_error())?;
    bytes.try_into().map_err(|_| incorrect_credential_error())
}

fn incorrect_credential_error() -> SetupLibraryError {
    SetupLibraryError::new(
        "incorrect_credentials",
        "The password or recovery answer is incorrect.",
    )
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

fn validate_existing_library(folder: &Path) -> Result<LibraryMarker, SetupLibraryError> {
    let metadata = fs::metadata(folder).map_err(|_| {
        SetupLibraryError::new(
            "folder_unavailable",
            "The selected library folder is unavailable.",
        )
    })?;
    if !metadata.is_dir() {
        return Err(SetupLibraryError::new(
            "not_a_folder",
            "Select a folder, not a file.",
        ));
    }
    let state_dir = folder.join(STATE_DIR);
    let marker_bytes = fs::read(state_dir.join(MARKER_FILE)).map_err(|_| {
        SetupLibraryError::new(
            "library_unrecognized",
            "This folder is not a recognized Photo Handler library.",
        )
    })?;
    let marker: LibraryMarker = serde_json::from_slice(&marker_bytes).map_err(|_| {
        SetupLibraryError::new(
            "library_unrecognized",
            "This folder has an invalid Photo Handler marker.",
        )
    })?;
    if marker.format_version != FORMAT_VERSION {
        return Err(SetupLibraryError::new(
            "library_version_unsupported",
            "This Photo Handler library uses an unsupported version.",
        ));
    }
    if !state_dir.join(DATABASE_FILE).is_file() {
        return Err(SetupLibraryError::new(
            "library_unrecognized",
            "This folder is missing its protected catalogue.",
        ));
    }
    Ok(marker)
}

fn validate_catalogue(
    path: &Path,
    database_key: &[u8; KEY_BYTES],
) -> Result<(), SetupLibraryError> {
    let connection = Connection::open(path).map_err(|_| incorrect_credential_error())?;
    connection
        .pragma_update(None, "key", format!("x'{}'", hex::encode(database_key)))
        .map_err(|_| incorrect_credential_error())?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|_| incorrect_credential_error())?;
    let version: u32 = transaction
        .query_row("SELECT version FROM schema_migrations LIMIT 1", [], |row| {
            row.get(0)
        })
        .map_err(|_| incorrect_credential_error())?;
    let identity_version: u32 = transaction
        .query_row(
            "SELECT format_version FROM library_identity WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| incorrect_credential_error())?;
    if version != identity_version || version > FORMAT_VERSION {
        return Err(SetupLibraryError::new(
            "library_version_unsupported",
            "This Photo Handler library uses an unsupported schema version.",
        ));
    }
    if version == 0 {
        transaction
            .execute(
                "UPDATE schema_migrations SET version = ?1",
                [FORMAT_VERSION],
            )
            .map_err(|_| {
                SetupLibraryError::new(
                    "library_migration_failed",
                    "Could not migrate the protected library catalogue.",
                )
            })?;
        transaction
            .execute(
                "UPDATE library_identity SET format_version = ?1 WHERE id = 1",
                [FORMAT_VERSION],
            )
            .map_err(|_| {
                SetupLibraryError::new(
                    "library_migration_failed",
                    "Could not migrate the protected library catalogue.",
                )
            })?;
    }
    transaction.commit().map_err(|_| {
        SetupLibraryError::new(
            "library_migration_failed",
            "Could not migrate the protected library catalogue.",
        )
    })?;
    Ok(())
}

fn write_marker(state_dir: &Path, marker: &LibraryMarker) -> Result<(), SetupLibraryError> {
    let bytes = serde_json::to_vec_pretty(marker).map_err(|_| {
        SetupLibraryError::new(
            "recovery_failed",
            "Could not update the library protection settings.",
        )
    })?;
    let temporary = state_dir.join("library.json.pending");
    fs::write(&temporary, bytes).map_err(|_| {
        SetupLibraryError::new(
            "recovery_failed",
            "Could not update the library protection settings.",
        )
    })?;
    fs::rename(&temporary, state_dir.join(MARKER_FILE)).map_err(|_| {
        let _ = fs::remove_file(&temporary);
        SetupLibraryError::new(
            "recovery_failed",
            "Could not update the library protection settings.",
        )
    })
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

    fn unlock_request(password: &str) -> UnlockLibraryRequest {
        UnlockLibraryRequest {
            password: password.into(),
        }
    }

    fn update_catalogue_versions(path: &Path, version: u32) {
        let marker: LibraryMarker =
            serde_json::from_slice(&fs::read(path.join(STATE_DIR).join(MARKER_FILE)).unwrap())
                .unwrap();
        let mut key = unwrap_key(&marker.password_wrap, "correct horse battery staple").unwrap();
        let connection = Connection::open(path.join(STATE_DIR).join(DATABASE_FILE)).unwrap();
        connection
            .pragma_update(None, "key", format!("x'{}'", hex::encode(key)))
            .unwrap();
        connection
            .execute("UPDATE schema_migrations SET version = ?1", [version])
            .unwrap();
        connection
            .execute(
                "UPDATE library_identity SET format_version = ?1 WHERE id = 1",
                [version],
            )
            .unwrap();
        key.fill(0);
    }

    #[test]
    fn correct_password_reopens_the_same_library() {
        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();

        let result = unlock_library(
            &directory.path().display().to_string(),
            unlock_request("correct horse battery staple"),
        )
        .unwrap();

        assert_eq!(result.folder_path, directory.path().display().to_string());
    }

    #[test]
    fn wrong_password_does_not_change_catalogue_or_marker() {
        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();
        let state = directory.path().join(STATE_DIR);
        let marker_before = fs::read(state.join(MARKER_FILE)).unwrap();
        let database_before = fs::read(state.join(DATABASE_FILE)).unwrap();

        let error = unlock_library(
            &directory.path().display().to_string(),
            unlock_request("wrong password"),
        )
        .unwrap_err();

        assert_eq!(error.code, "incorrect_credentials");
        assert_eq!(fs::read(state.join(MARKER_FILE)).unwrap(), marker_before);
        assert_eq!(
            fs::read(state.join(DATABASE_FILE)).unwrap(),
            database_before
        );
    }

    #[test]
    fn recovery_answer_resets_password_without_replacing_catalogue() {
        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();
        let database = directory.path().join(STATE_DIR).join(DATABASE_FILE);
        let before = fs::read(&database).unwrap();

        reset_library_password(
            &directory.path().display().to_string(),
            ResetLibraryPasswordRequest {
                folder_path: directory.path().display().to_string(),
                recovery_answer: "Mochi".into(),
                new_password: "new password".into(),
                new_password_confirmation: "new password".into(),
            },
        )
        .unwrap();

        assert_eq!(fs::read(database).unwrap(), before);
        assert_eq!(
            unlock_library(
                &directory.path().display().to_string(),
                unlock_request("correct horse battery staple")
            )
            .unwrap_err()
            .code,
            "incorrect_credentials"
        );
        assert!(unlock_library(
            &directory.path().display().to_string(),
            unlock_request("new password")
        )
        .is_ok());
    }

    #[test]
    fn unlock_migrates_a_supported_older_catalogue_schema() {
        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();
        update_catalogue_versions(directory.path(), 0);

        unlock_library(
            &directory.path().display().to_string(),
            unlock_request("correct horse battery staple"),
        )
        .unwrap();

        let marker: LibraryMarker = serde_json::from_slice(
            &fs::read(directory.path().join(STATE_DIR).join(MARKER_FILE)).unwrap(),
        )
        .unwrap();
        let mut key = unwrap_key(&marker.password_wrap, "correct horse battery staple").unwrap();
        let connection =
            Connection::open(directory.path().join(STATE_DIR).join(DATABASE_FILE)).unwrap();
        connection
            .pragma_update(None, "key", format!("x'{}'", hex::encode(key)))
            .unwrap();
        let version: u32 = connection
            .query_row("SELECT version FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        key.fill(0);
        assert_eq!(version, FORMAT_VERSION);
    }

    #[test]
    fn newer_catalogue_schema_is_rejected_without_mutation() {
        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();
        update_catalogue_versions(directory.path(), FORMAT_VERSION + 1);
        let database = directory.path().join(STATE_DIR).join(DATABASE_FILE);
        let before = fs::read(&database).unwrap();

        let error = unlock_library(
            &directory.path().display().to_string(),
            unlock_request("correct horse battery staple"),
        )
        .unwrap_err();

        assert_eq!(error.code, "library_version_unsupported");
        assert_eq!(fs::read(database).unwrap(), before);
    }

    #[test]
    fn wrong_recovery_answer_preserves_all_library_files() {
        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();
        let state = directory.path().join(STATE_DIR);
        let marker_before = fs::read(state.join(MARKER_FILE)).unwrap();
        let database_before = fs::read(state.join(DATABASE_FILE)).unwrap();

        let error = reset_library_password(
            &directory.path().display().to_string(),
            ResetLibraryPasswordRequest {
                folder_path: directory.path().display().to_string(),
                recovery_answer: "wrong answer".into(),
                new_password: "new password".into(),
                new_password_confirmation: "new password".into(),
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "incorrect_credentials");
        assert_eq!(fs::read(state.join(MARKER_FILE)).unwrap(), marker_before);
        assert_eq!(
            fs::read(state.join(DATABASE_FILE)).unwrap(),
            database_before
        );
    }

    #[test]
    fn stale_pointer_and_foreign_or_unsupported_libraries_are_rejected_without_mutation() {
        let settings = tempdir().unwrap();
        remember_library_path(settings.path(), "/missing/photo-handler-library").unwrap();
        assert_eq!(remembered_library(settings.path()).unwrap().state, "stale");

        let foreign = tempdir().unwrap();
        fs::create_dir(foreign.path().join(STATE_DIR)).unwrap();
        fs::write(
            foreign.path().join(STATE_DIR).join(MARKER_FILE),
            b"not json",
        )
        .unwrap();
        let foreign_before = fs::read(foreign.path().join(STATE_DIR).join(MARKER_FILE)).unwrap();
        assert_eq!(
            unlock_library(
                &foreign.path().display().to_string(),
                unlock_request("password")
            )
            .unwrap_err()
            .code,
            "library_unrecognized"
        );
        assert_eq!(
            fs::read(foreign.path().join(STATE_DIR).join(MARKER_FILE)).unwrap(),
            foreign_before
        );

        let unsupported = tempdir().unwrap();
        setup_library(request(unsupported.path())).unwrap();
        let marker_path = unsupported.path().join(STATE_DIR).join(MARKER_FILE);
        let mut marker: serde_json::Value =
            serde_json::from_slice(&fs::read(&marker_path).unwrap()).unwrap();
        marker["format_version"] = serde_json::json!(2);
        fs::write(&marker_path, serde_json::to_vec(&marker).unwrap()).unwrap();
        assert_eq!(
            unlock_library(
                &unsupported.path().display().to_string(),
                unlock_request("correct horse battery staple")
            )
            .unwrap_err()
            .code,
            "library_version_unsupported"
        );
    }
}

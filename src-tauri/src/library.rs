use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::{rngs::OsRng, RngCore};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::import_source;

const STATE_DIR: &str = ".photo-handler";
const MARKER_FILE: &str = "library.json";
const DATABASE_FILE: &str = "catalogue.db";
const LIBRARY_POINTER_FILE: &str = "selected-library.json";
const MARKER_FORMAT_VERSION: u32 = 1;
const CATALOGUE_FORMAT_VERSION: u32 = 8;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanLibraryRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanLibraryResult {
    pub moved_folder_count: usize,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failed_targets: Vec<String>,
}

impl SetupLibraryError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            failed_targets: Vec::new(),
        }
    }

    fn cleanup_incomplete(failed_targets: Vec<String>) -> Self {
        let target_summary = failed_targets.join(", ");
        Self {
            code: "cleanup_incomplete",
            message: format!(
                "Could not move {target_summary} to the operating system Trash. Review data and the remembered import folder were kept so you can retry."
            ),
            failed_targets,
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

struct AuthenticatedSession {
    library_path: PathBuf,
    database_key: [u8; KEY_BYTES],
}

impl Drop for AuthenticatedSession {
    fn drop(&mut self) {
        self.database_key.fill(0);
    }
}

fn active_session() -> &'static Mutex<Option<AuthenticatedSession>> {
    static SESSION: OnceLock<Mutex<Option<AuthenticatedSession>>> = OnceLock::new();
    SESSION.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn test_session_guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn establish_session(library_path: &Path, database_key: [u8; KEY_BYTES]) {
    *active_session()
        .lock()
        .expect("library session mutex poisoned") = Some(AuthenticatedSession {
        library_path: library_path.to_path_buf(),
        database_key,
    });
}

pub fn lock_library() {
    *active_session()
        .lock()
        .expect("library session mutex poisoned") = None;
}

pub fn active_library_path() -> Result<PathBuf, SetupLibraryError> {
    let guard = active_session().lock().map_err(|_| {
        SetupLibraryError::new(
            "library_locked",
            "The protected library is locked. Unlock it before opening its folder.",
        )
    })?;
    let session = guard.as_ref().ok_or_else(|| {
        SetupLibraryError::new(
            "library_locked",
            "The protected library is locked. Unlock it before opening its folder.",
        )
    })?;
    validate_existing_library(&session.library_path)?;
    Ok(session.library_path.clone())
}

pub fn clean_library(
    request: CleanLibraryRequest,
    app_data_dir: &Path,
) -> Result<CleanLibraryResult, SetupLibraryError> {
    clean_library_with_trash(request, app_data_dir, |target| {
        trash::delete(target).map_err(|error| error.to_string())
    })
}

fn clean_library_with_trash(
    mut request: CleanLibraryRequest,
    app_data_dir: &Path,
    move_to_trash: impl Fn(&Path) -> Result<(), String>,
) -> Result<CleanLibraryResult, SetupLibraryError> {
    if request.password.is_empty() {
        return Err(SetupLibraryError::new(
            "missing_password",
            "Enter your current library password to clean managed copies.",
        ));
    }

    let result = with_catalogue(|connection, library_path| {
        let marker = validate_existing_library(library_path)?;
        let mut verified_key = unwrap_key(&marker.password_wrap, &request.password)?;
        verified_key.fill(0);
        request.password.clear();

        let targets = managed_media_folders(library_path)?;
        let mut failed_targets = Vec::new();
        for target in &targets {
            if move_to_trash(target).is_err() {
                // Only expose the validated date-folder name, never an arbitrary filesystem path.
                failed_targets.push(
                    target
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        if !failed_targets.is_empty() {
            return Err(SetupLibraryError::cleanup_incomplete(failed_targets));
        }

        let transaction = connection.unchecked_transaction().map_err(|error| {
            SetupLibraryError::new(
                "cleanup_failed",
                format!("Could not clear review data: {error}"),
            )
        })?;
        transaction
            .execute_batch(
                "DELETE FROM candidate_tags; DELETE FROM tags; DELETE FROM item_decisions; DELETE FROM review_candidates; DELETE FROM review_sessions;",
            )
            .map_err(|error| SetupLibraryError::new("cleanup_failed", format!("Could not clear review data: {error}")))?;
        transaction.commit().map_err(|error| {
            SetupLibraryError::new(
                "cleanup_failed",
                format!("Could not clear review data: {error}"),
            )
        })?;
        import_source::clear_remembered_import_source(app_data_dir)
            .map_err(|error| SetupLibraryError::new(error.code, error.message))?;

        Ok(CleanLibraryResult {
            moved_folder_count: targets.len(),
            message: "Managed date folders were moved to the operating system Trash. Review history and the remembered import folder were cleared; protected library setup remains unlocked.".into(),
        })
    });
    request.password.clear();
    result
}

pub fn with_catalogue<T, E: From<SetupLibraryError>>(
    callback: impl FnOnce(&Connection, &Path) -> Result<T, E>,
) -> Result<T, E> {
    let guard = active_session().lock().map_err(|_| {
        E::from(SetupLibraryError::new(
            "library_locked",
            "The protected library is locked. Unlock it before reviewing media.",
        ))
    })?;
    let session = guard.as_ref().ok_or_else(|| {
        E::from(SetupLibraryError::new(
            "library_locked",
            "The protected library is locked. Unlock it before reviewing media.",
        ))
    })?;
    validate_existing_library(&session.library_path).map_err(E::from)?;
    let connection = Connection::open(session.library_path.join(STATE_DIR).join(DATABASE_FILE))
        .map_err(|_| {
            E::from(SetupLibraryError::new(
                "library_locked",
                "The protected library is unavailable. Unlock it again before reviewing media.",
            ))
        })?;
    connection
        .pragma_update(
            None,
            "key",
            format!("x'{}'", hex::encode(session.database_key)),
        )
        .map_err(|_| {
            E::from(SetupLibraryError::new(
                "library_locked",
                "The protected library is locked. Unlock it before reviewing media.",
            ))
        })?;
    callback(&connection, &session.library_path)
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
            format_version: MARKER_FORMAT_VERSION,
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
        establish_session(&folder, database_key);
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
    let pointer =
        serde_json::json!({ "format_version": MARKER_FORMAT_VERSION, "folder_path": folder });
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
    if pointer.format_version != MARKER_FORMAT_VERSION || pointer.folder_path.trim().is_empty() {
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
    let database_key = unwrap_key(&marker.password_wrap, &request.password)?;
    validate_catalogue(&folder.join(STATE_DIR).join(DATABASE_FILE), &database_key)?;
    establish_session(&folder, database_key);
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
    let database_key = unwrap_key(&marker.recovery_wrap, &request.recovery_answer)?;
    validate_catalogue(&folder.join(STATE_DIR).join(DATABASE_FILE), &database_key)?;
    let password_wrap = wrap_key(&database_key, &request.new_password)?;
    marker.password_wrap = password_wrap;
    write_marker(&folder.join(STATE_DIR), &marker)?;
    establish_session(&folder, database_key);
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
    connection.execute_batch("BEGIN IMMEDIATE; CREATE TABLE schema_migrations (version INTEGER NOT NULL); INSERT INTO schema_migrations (version) VALUES (6); CREATE TABLE library_identity (id INTEGER PRIMARY KEY CHECK (id = 1), format_version INTEGER NOT NULL); INSERT INTO library_identity (id, format_version) VALUES (1, 6); CREATE TABLE review_sessions (id INTEGER PRIMARY KEY, source_path TEXT NOT NULL UNIQUE, state TEXT NOT NULL CHECK (state IN ('active', 'complete')), created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP); CREATE TABLE review_candidates (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES review_sessions(id), relative_path TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1, file_size INTEGER NOT NULL, modified_at INTEGER NOT NULL, media_type TEXT NOT NULL, content_fingerprint_algorithm TEXT NULL, content_fingerprint_value BLOB NULL, decision TEXT NULL CHECK (decision IN ('imported', 'skipped')), UNIQUE(session_id, relative_path, revision)); CREATE TABLE tags (id INTEGER PRIMARY KEY, normalized_name TEXT NOT NULL UNIQUE); CREATE TABLE candidate_tags (candidate_id INTEGER NOT NULL REFERENCES review_candidates(id), tag_id INTEGER NOT NULL REFERENCES tags(id), PRIMARY KEY(candidate_id, tag_id)); CREATE TABLE item_decisions (candidate_id INTEGER PRIMARY KEY REFERENCES review_candidates(id), decision TEXT NOT NULL CHECK (decision IN ('imported', 'skipped')), decided_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, destination_path TEXT NULL, effective_import_date TEXT NULL, date_origin TEXT NULL, original_media_date TEXT NULL, original_date_origin TEXT NULL); CREATE INDEX item_decisions_imported_date_idx ON item_decisions(decision, effective_import_date); CREATE INDEX item_decisions_original_date_idx ON item_decisions(decision, original_media_date); CREATE INDEX review_candidates_media_type_idx ON review_candidates(media_type); CREATE INDEX review_candidates_content_fingerprint_idx ON review_candidates(content_fingerprint_algorithm, content_fingerprint_value); COMMIT;")
        .map_err(|error| SetupLibraryError::new("initialization_failed", format!("Could not initialize catalogue schema: {error}")))?;
    connection.execute_batch("ALTER TABLE review_candidates ADD COLUMN perceptual_hash_algorithm TEXT NULL; ALTER TABLE review_candidates ADD COLUMN perceptual_hash_value INTEGER NULL; ALTER TABLE review_candidates ADD COLUMN visual_comparison_state TEXT NULL; CREATE INDEX review_candidates_perceptual_hash_idx ON review_candidates(perceptual_hash_algorithm, perceptual_hash_value); UPDATE schema_migrations SET version = 7; UPDATE library_identity SET format_version = 7;")
        .map_err(|error| SetupLibraryError::new("initialization_failed", format!("Could not initialize catalogue schema: {error}")))?;
    connection.execute_batch("ALTER TABLE review_candidates ADD COLUMN perceptual_hash_threshold INTEGER NULL; UPDATE schema_migrations SET version = 8; UPDATE library_identity SET format_version = 8;")
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
    if marker.format_version != MARKER_FORMAT_VERSION {
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

fn managed_media_folders(library_path: &Path) -> Result<Vec<PathBuf>, SetupLibraryError> {
    let entries = fs::read_dir(library_path).map_err(|error| {
        SetupLibraryError::new(
            "library_unavailable",
            format!("Could not inspect the protected library: {error}"),
        )
    })?;
    let mut targets = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            SetupLibraryError::new(
                "library_unavailable",
                format!("Could not inspect the protected library: {error}"),
            )
        })?;
        if entry.file_name() == STATE_DIR {
            continue;
        }
        let kind = entry.file_type().map_err(|error| {
            SetupLibraryError::new(
                "library_unavailable",
                format!("Could not inspect the protected library: {error}"),
            )
        })?;
        if !kind.is_dir() || kind.is_symlink() || !is_year_name(&entry.file_name()) {
            continue;
        }
        let path = entry.path();
        let date_folders = managed_date_folders(&path)?;
        if !date_folders.is_empty() {
            targets.extend(date_folders);
        }
    }
    targets.sort();
    Ok(targets)
}

fn is_year_name(name: &std::ffi::OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    name.len() == 4 && name.bytes().all(|byte| byte.is_ascii_digit())
}

fn managed_date_folders(path: &Path) -> Result<Vec<PathBuf>, SetupLibraryError> {
    let entries = fs::read_dir(path).map_err(|error| {
        SetupLibraryError::new(
            "library_unavailable",
            format!("Could not inspect a managed-media folder: {error}"),
        )
    })?;
    let mut date_folders = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            SetupLibraryError::new(
                "library_unavailable",
                format!("Could not inspect a managed-media folder: {error}"),
            )
        })?;
        let kind = entry.file_type().map_err(|error| {
            SetupLibraryError::new(
                "library_unavailable",
                format!("Could not inspect a managed-media folder: {error}"),
            )
        })?;
        if kind.is_file() && entry.file_name() == ".DS_Store" {
            continue;
        }
        if !kind.is_dir() || kind.is_symlink() || !is_date_folder_name(&entry.file_name()) {
            return Ok(Vec::new());
        }
        date_folders.push(entry.path());
    }
    Ok(date_folders)
}

fn is_date_folder_name(name: &std::ffi::OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    name.len() == 10
        && name.as_bytes()[4] == b'-'
        && name.as_bytes()[7] == b'-'
        && name
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
        && chrono::NaiveDate::parse_from_str(name, "%Y-%m-%d").is_ok()
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
    if version != identity_version || version > CATALOGUE_FORMAT_VERSION {
        return Err(SetupLibraryError::new(
            "library_version_unsupported",
            "This Photo Handler library uses an unsupported schema version.",
        ));
    }
    if version < CATALOGUE_FORMAT_VERSION {
        transaction.execute_batch("CREATE TABLE IF NOT EXISTS review_sessions (id INTEGER PRIMARY KEY, source_path TEXT NOT NULL UNIQUE, state TEXT NOT NULL CHECK (state IN ('active', 'complete')), created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP); CREATE TABLE IF NOT EXISTS review_candidates (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES review_sessions(id), relative_path TEXT NOT NULL, file_size INTEGER NOT NULL, modified_at INTEGER NOT NULL, media_type TEXT NOT NULL, decision TEXT NULL CHECK (decision IN ('imported', 'skipped')), UNIQUE(session_id, relative_path)); CREATE TABLE IF NOT EXISTS tags (id INTEGER PRIMARY KEY, normalized_name TEXT NOT NULL UNIQUE); CREATE TABLE IF NOT EXISTS candidate_tags (candidate_id INTEGER NOT NULL REFERENCES review_candidates(id), tag_id INTEGER NOT NULL REFERENCES tags(id), PRIMARY KEY(candidate_id, tag_id)); CREATE TABLE IF NOT EXISTS item_decisions (candidate_id INTEGER PRIMARY KEY REFERENCES review_candidates(id), decision TEXT NOT NULL CHECK (decision IN ('imported', 'skipped')), decided_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, destination_path TEXT NULL);")
            .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
        if version < 3 {
            let date_column_exists: i64 = transaction
                .query_row(
                    "SELECT count(*) FROM pragma_table_info('item_decisions') WHERE name = 'effective_import_date'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
            if date_column_exists == 0 {
                transaction
                    .execute_batch("ALTER TABLE item_decisions ADD COLUMN effective_import_date TEXT NULL; ALTER TABLE item_decisions ADD COLUMN date_origin TEXT NULL;")
                    .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
            }
        }
        if version < 4 {
            transaction
                .execute_batch("CREATE INDEX IF NOT EXISTS item_decisions_imported_date_idx ON item_decisions(decision, effective_import_date); CREATE INDEX IF NOT EXISTS review_candidates_media_type_idx ON review_candidates(media_type);")
                .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
        }
        if version < 5 {
            let original_date_column_exists: i64 = transaction
                .query_row(
                    "SELECT count(*) FROM pragma_table_info('item_decisions') WHERE name = 'original_media_date'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
            if original_date_column_exists == 0 {
                transaction
                    .execute_batch("ALTER TABLE item_decisions ADD COLUMN original_media_date TEXT NULL; ALTER TABLE item_decisions ADD COLUMN original_date_origin TEXT NULL;")
                    .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
            }
            transaction
                .execute_batch("CREATE INDEX IF NOT EXISTS item_decisions_original_date_idx ON item_decisions(decision, original_media_date);")
                .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
        }
        if version < 6 {
            transaction.execute_batch("CREATE TABLE review_candidates_v6 (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES review_sessions(id), relative_path TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1, file_size INTEGER NOT NULL, modified_at INTEGER NOT NULL, media_type TEXT NOT NULL, content_fingerprint_algorithm TEXT NULL, content_fingerprint_value BLOB NULL, decision TEXT NULL CHECK (decision IN ('imported', 'skipped')), UNIQUE(session_id, relative_path, revision)); INSERT INTO review_candidates_v6 (id, session_id, relative_path, revision, file_size, modified_at, media_type, decision) SELECT id, session_id, relative_path, 1, file_size, modified_at, media_type, decision FROM review_candidates; DROP TABLE review_candidates; ALTER TABLE review_candidates_v6 RENAME TO review_candidates; CREATE INDEX review_candidates_media_type_idx ON review_candidates(media_type); CREATE INDEX review_candidates_content_fingerprint_idx ON review_candidates(content_fingerprint_algorithm, content_fingerprint_value);")
                .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
        }
        if version < 7 {
            transaction.execute_batch("ALTER TABLE review_candidates ADD COLUMN perceptual_hash_algorithm TEXT NULL; ALTER TABLE review_candidates ADD COLUMN perceptual_hash_value INTEGER NULL; ALTER TABLE review_candidates ADD COLUMN visual_comparison_state TEXT NULL; CREATE INDEX IF NOT EXISTS review_candidates_perceptual_hash_idx ON review_candidates(perceptual_hash_algorithm, perceptual_hash_value);")
                .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
        }
        if version < 8 {
            transaction
                .execute_batch("ALTER TABLE review_candidates ADD COLUMN perceptual_hash_threshold INTEGER NULL;")
                .map_err(|_| SetupLibraryError::new("library_migration_failed", "Could not migrate the protected library catalogue."))?;
        }
        transaction
            .execute(
                "UPDATE schema_migrations SET version = ?1",
                [CATALOGUE_FORMAT_VERSION],
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
                [CATALOGUE_FORMAT_VERSION],
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
    use std::cell::Cell;
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
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
        let settings = tempdir().unwrap();
        remember_library_path(settings.path(), "/example/library").unwrap();
        let pointer: serde_json::Value =
            serde_json::from_slice(&fs::read(settings.path().join(LIBRARY_POINTER_FILE)).unwrap())
                .unwrap();
        assert_eq!(pointer["folder_path"], "/example/library");
    }

    #[test]
    fn invalid_request_leaves_target_untouched() {
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
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
    fn active_library_path_requires_an_unlocked_available_library() {
        let _session_guard = test_session_guard();
        lock_library();
        assert_eq!(active_library_path().unwrap_err().code, "library_locked");

        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();
        assert_eq!(active_library_path().unwrap(), directory.path());

        fs::remove_dir_all(directory.path()).unwrap();
        assert_eq!(
            active_library_path().unwrap_err().code,
            "folder_unavailable"
        );
        lock_library();
    }

    #[test]
    fn wrong_password_does_not_change_catalogue_or_marker() {
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
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
        let threshold_column_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM pragma_table_info('review_candidates') WHERE name = 'perceptual_hash_threshold'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        key.fill(0);
        assert_eq!(version, CATALOGUE_FORMAT_VERSION);
        assert_eq!(threshold_column_count, 1);
    }

    #[test]
    fn newer_catalogue_schema_is_rejected_without_mutation() {
        let _session_guard = test_session_guard();
        let directory = tempdir().unwrap();
        setup_library(request(directory.path())).unwrap();
        update_catalogue_versions(directory.path(), CATALOGUE_FORMAT_VERSION + 1);
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
        let _session_guard = test_session_guard();
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
        let _session_guard = test_session_guard();
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

    #[test]
    fn managed_media_classification_ignores_finder_metadata_but_rejects_other_content() {
        let library = tempdir().unwrap();
        let managed_year = library.path().join("2026");
        fs::create_dir(&managed_year).unwrap();
        fs::create_dir(managed_year.join("2026-08-28")).unwrap();
        fs::write(managed_year.join("2026-08-28").join("copy.jpg"), b"copy").unwrap();
        fs::write(managed_year.join(".DS_Store"), b"finder metadata").unwrap();

        let mixed_year = library.path().join("2025");
        fs::create_dir(&mixed_year).unwrap();
        fs::create_dir(mixed_year.join("2025-01-01")).unwrap();
        fs::write(mixed_year.join("notes.txt"), b"preserve").unwrap();
        let malformed_year = library.path().join("2024");
        fs::create_dir(&malformed_year).unwrap();
        fs::create_dir(malformed_year.join("2024-13-40")).unwrap();
        fs::create_dir(library.path().join("not-a-year")).unwrap();

        assert_eq!(
            managed_media_folders(library.path()).unwrap(),
            vec![managed_year.join("2026-08-28")]
        );
    }

    #[cfg(unix)]
    #[test]
    fn managed_media_classification_rejects_symlinked_date_paths() {
        use std::os::unix::fs::symlink;

        let library = tempdir().unwrap();
        let year = library.path().join("2026");
        let outside = tempdir().unwrap();
        fs::create_dir(&year).unwrap();
        symlink(outside.path(), year.join("2026-08-28")).unwrap();

        assert!(managed_media_folders(library.path()).unwrap().is_empty());
        assert!(outside.path().exists());
    }

    #[test]
    fn clean_library_rejects_locked_and_wrong_password_without_changing_media() {
        let _session_guard = test_session_guard();
        let library = tempdir().unwrap();
        let settings = tempdir().unwrap();
        setup_library(request(library.path())).unwrap();
        let managed_year = library.path().join("2026");
        fs::create_dir(&managed_year).unwrap();
        fs::create_dir(managed_year.join("2026-08-28")).unwrap();

        lock_library();
        assert_eq!(
            clean_library(
                CleanLibraryRequest {
                    password: "correct horse battery staple".into(),
                },
                settings.path(),
            )
            .unwrap_err()
            .code,
            "library_locked"
        );
        assert!(managed_year.exists());

        unlock_library(
            &library.path().display().to_string(),
            unlock_request("correct horse battery staple"),
        )
        .unwrap();
        assert_eq!(
            clean_library(
                CleanLibraryRequest {
                    password: "wrong password".into(),
                },
                settings.path(),
            )
            .unwrap_err()
            .code,
            "incorrect_credentials"
        );
        assert!(managed_year.exists());
    }

    fn seed_review_data_and_source(library: &Path, settings: &Path, source: &Path) {
        import_source::save_import_source(
            settings,
            &library.display().to_string(),
            import_source::SaveImportSourceRequest {
                folder_path: source.display().to_string(),
            },
        )
        .unwrap();
        with_catalogue(|connection, _| {
            connection.execute_batch(
                "INSERT INTO review_sessions (id, source_path, state) VALUES (1, '/source', 'complete');\
                 INSERT INTO review_candidates (id, session_id, relative_path, file_size, modified_at, media_type) VALUES (1, 1, 'copy.jpg', 1, 1, 'image');\
                 INSERT INTO tags (id, normalized_name) VALUES (1, 'debug');\
                 INSERT INTO candidate_tags (candidate_id, tag_id) VALUES (1, 1);\
                 INSERT INTO item_decisions (candidate_id, decision, destination_path) VALUES (1, 'imported', '/missing/old-copy.jpg');",
            )
            .map_err(|error| SetupLibraryError::new("test_failed", error.to_string()))?;
            Ok::<_, SetupLibraryError>(())
        })
        .unwrap();
    }

    fn review_row_count() -> i64 {
        with_catalogue(|connection, _| {
            connection.query_row(
                "SELECT (SELECT count(*) FROM review_sessions) + (SELECT count(*) FROM review_candidates) + (SELECT count(*) FROM tags) + (SELECT count(*) FROM candidate_tags) + (SELECT count(*) FROM item_decisions)",
                [],
                |row| row.get(0),
            ).map_err(|error| SetupLibraryError::new("test_failed", error.to_string()))
        })
        .unwrap()
    }

    #[test]
    fn partial_cleanup_keeps_metadata_and_source_until_a_successful_retry() {
        let _session_guard = test_session_guard();
        let library = tempdir().unwrap();
        let settings = tempdir().unwrap();
        let source = tempdir().unwrap();
        let original = source.path().join("original.jpg");
        fs::write(&original, b"original").unwrap();
        setup_library(request(library.path())).unwrap();
        let first = library.path().join("2026").join("2026-08-28");
        let second = library.path().join("2026").join("2026-08-30");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(first.join("copy.jpg"), b"copy").unwrap();
        fs::write(second.join("copy.jpg"), b"copy").unwrap();
        let unrelated = library.path().join("notes.txt");
        fs::write(&unrelated, b"preserve").unwrap();

        unlock_library(
            &library.path().display().to_string(),
            unlock_request("correct horse battery staple"),
        )
        .unwrap();
        seed_review_data_and_source(library.path(), settings.path(), source.path());

        let moves = Cell::new(0);
        let error = clean_library_with_trash(
            CleanLibraryRequest {
                password: "correct horse battery staple".into(),
            },
            settings.path(),
            |target| {
                moves.set(moves.get() + 1);
                if target == second {
                    Err("simulated Trash failure".into())
                } else {
                    fs::remove_dir_all(target).map_err(|error| error.to_string())
                }
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "cleanup_incomplete");
        assert_eq!(error.failed_targets, vec!["2026-08-30"]);
        assert_eq!(moves.get(), 2);
        assert!(!first.exists());
        assert!(second.exists());
        assert_eq!(review_row_count(), 5);
        assert_eq!(
            import_source::remembered_import_source(settings.path())
                .unwrap()
                .state,
            "ready"
        );
        assert_eq!(fs::read(&original).unwrap(), b"original");
        assert_eq!(fs::read(&unrelated).unwrap(), b"preserve");

        let result = clean_library_with_trash(
            CleanLibraryRequest {
                password: "correct horse battery staple".into(),
            },
            settings.path(),
            |target| fs::remove_dir_all(target).map_err(|error| error.to_string()),
        )
        .unwrap();
        assert_eq!(result.moved_folder_count, 1);
        assert!(!second.exists());
        assert_eq!(review_row_count(), 0);
        assert_eq!(
            import_source::remembered_import_source(settings.path())
                .unwrap()
                .state,
            "missing"
        );
        assert!(library.path().join(STATE_DIR).join(MARKER_FILE).is_file());
        assert!(library.path().join(STATE_DIR).join(DATABASE_FILE).is_file());
        lock_library();
        assert!(unlock_library(
            &library.path().display().to_string(),
            unlock_request("correct horse battery staple"),
        )
        .is_ok());
    }

    #[test]
    fn empty_cleanup_clears_only_review_state_and_ignores_stale_destinations() {
        let _session_guard = test_session_guard();
        let library = tempdir().unwrap();
        let settings = tempdir().unwrap();
        let source = tempdir().unwrap();
        let original = source.path().join("original.jpg");
        fs::write(&original, b"original").unwrap();
        setup_library(request(library.path())).unwrap();
        unlock_library(
            &library.path().display().to_string(),
            unlock_request("correct horse battery staple"),
        )
        .unwrap();
        seed_review_data_and_source(library.path(), settings.path(), source.path());

        let result = clean_library_with_trash(
            CleanLibraryRequest {
                password: "correct horse battery staple".into(),
            },
            settings.path(),
            |_| panic!("an empty library must not invoke Trash"),
        )
        .unwrap();
        assert_eq!(result.moved_folder_count, 0);
        assert_eq!(review_row_count(), 0);
        assert_eq!(
            import_source::remembered_import_source(settings.path())
                .unwrap()
                .state,
            "missing"
        );
        assert_eq!(fs::read(&original).unwrap(), b"original");
    }
}

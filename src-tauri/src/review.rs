use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Local};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::library::{self, SetupLibraryError};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewState {
    pub state: &'static str,
    pub source_path: Option<String>,
    pub candidate_count: u64,
    pub message: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewItem {
    pub state: &'static str,
    pub candidate_id: Option<i64>,
    pub relative_path: Option<String>,
    pub filename: Option<String>,
    pub media_type: Option<String>,
    pub effective_import_date: Option<String>,
    pub date_origin: Option<String>,
    pub tags: Vec<String>,
    pub preview_url: Option<String>,
    pub message: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecideRequest {
    pub candidate_id: i64,
    pub tags: Vec<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub candidate_id: i64,
    pub tags: Vec<String>,
    pub effective_import_date: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionResult {
    pub decision: &'static str,
    pub destination_path: Option<String>,
    pub message: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewError {
    pub code: &'static str,
    pub message: String,
}
impl From<SetupLibraryError> for ReviewError {
    fn from(value: SetupLibraryError) -> Self {
        Self {
            code: value.code,
            message: value.message,
        }
    }
}

#[derive(Clone)]
struct Candidate {
    relative_path: String,
    file_size: i64,
    modified_at: i64,
    media_type: &'static str,
}

pub fn current_review_state() -> Result<ReviewState, ReviewError> {
    library::with_catalogue(|connection, _| review_state(connection))
}

pub fn start_review(source_path: &str) -> Result<ReviewState, ReviewError> {
    library::with_catalogue(|connection, library_path| {
        let source = canonical_directory(source_path)?;
        let library = library_path.canonicalize().map_err(|_| {
            error(
                "library_unavailable",
                "The protected library is unavailable.",
            )
        })?;
        if source == library || source.starts_with(&library) || library.starts_with(&source) {
            return Err(error("source_overlaps_library", "The import source must be separate from the protected library. Nothing was changed."));
        }
        let source_text = source.display().to_string();
        if let Ok((id, count)) = connection.query_row("SELECT id, (SELECT count(*) FROM review_candidates WHERE session_id = review_sessions.id AND decision IS NULL) FROM review_sessions WHERE source_path = ?1 AND state = 'active'", [&source_text], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, u64>(1)?))) { return Ok(ReviewState { state: "resumable", source_path: Some(source_text), candidate_count: count, message: format!("Resuming the existing review session ({id}). Originals are never modified.") }); }
        let candidates = discover(&source)?;
        let transaction = connection.unchecked_transaction().map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO review_sessions (source_path, state) VALUES (?1, 'active')",
                [&source_text],
            )
            .map_err(database_error)?;
        let session_id = transaction.last_insert_rowid();
        for candidate in &candidates {
            transaction.execute("INSERT INTO review_candidates (session_id, relative_path, file_size, modified_at, media_type) VALUES (?1, ?2, ?3, ?4, ?5)", params![session_id, candidate.relative_path, candidate.file_size, candidate.modified_at, candidate.media_type]).map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(ReviewState {
            state: if candidates.is_empty() {
                "empty"
            } else {
                "started"
            },
            source_path: Some(source_text),
            candidate_count: candidates.len() as u64,
            message: "Discovery reads supported files only; it never modifies originals.".into(),
        })
    })
}

pub fn next_review_item(app: tauri::AppHandle) -> Result<ReviewItem, ReviewError> {
    library::with_catalogue(|connection, _| {
        let row = connection.query_row("SELECT c.id, s.source_path, c.relative_path, c.media_type FROM review_candidates c JOIN review_sessions s ON s.id = c.session_id WHERE s.state = 'active' AND c.decision IS NULL ORDER BY s.id DESC, c.relative_path LIMIT 1", [], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)));
        let (candidate_id, source_path, relative_path, media_type) = match row {
            Ok(row) => row,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(empty_item()),
            Err(error) => return Err(database_error(error)),
        };
        let path = Path::new(&source_path).join(&relative_path);
        let filename = Path::new(&relative_path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| relative_path.clone());
        let tags = candidate_tags(connection, candidate_id)?;
        let (date, origin) =
            effective_date(&path).unwrap_or_else(|| ("1970-01-01".into(), "unavailable".into()));
        let preview_url = match readable_file(&path) {
            Ok(()) => {
                app.asset_protocol_scope().allow_file(&path).map_err(|_| {
                    error(
                        "preview_unavailable",
                        "The selected item could not be prepared for in-app preview.",
                    )
                })?;
                Some(asset_url(&path)?)
            }
            Err(message) => {
                return Ok(ReviewItem {
                    state: "unavailable",
                    candidate_id: Some(candidate_id),
                    relative_path: Some(relative_path),
                    filename: Some(filename),
                    media_type: Some(media_type),
                    effective_import_date: Some(date),
                    date_origin: Some(origin),
                    tags,
                    preview_url: None,
                    message,
                })
            }
        };
        Ok(ReviewItem {
            state: "item",
            candidate_id: Some(candidate_id),
            relative_path: Some(relative_path),
            filename: Some(filename),
            media_type: Some(media_type),
            effective_import_date: Some(date),
            date_origin: Some(origin),
            tags,
            preview_url,
            message:
                "Review this item before choosing Import or Skip. Originals are never modified."
                    .into(),
        })
    })
}

pub fn skip_review_item(request: DecideRequest) -> Result<DecisionResult, ReviewError> {
    library::with_catalogue(|connection, _| {
        record_decision(
            connection,
            request.candidate_id,
            "skipped",
            &request.tags,
            None,
            None,
        )?;
        Ok(DecisionResult {
            decision: "skipped",
            destination_path: None,
            message: "Item skipped. The original file was not changed.".into(),
        })
    })
}
pub fn import_review_item(request: ImportRequest) -> Result<DecisionResult, ReviewError> {
    let date = validate_date(&request.effective_import_date)?;
    library::with_catalogue(|connection, library_path| {
        let (source_path, relative_path) = pending_candidate(connection, request.candidate_id)?;
        let source = Path::new(&source_path).join(relative_path);
        readable_file(&source).map_err(|message| error("candidate_unavailable", message))?;
        let destination = publish_copy(&source, library_path, &date)?;
        if let Err(record_error) = record_decision(
            connection,
            request.candidate_id,
            "imported",
            &request.tags,
            Some(&destination),
            Some(&date),
        ) {
            return Err(error("post_copy_catalogue_failure", format!("The file was copied to {} but its catalogue decision could not be saved. Do not import it again; reopen the library and recover this item. ({})", destination.display(), record_error.message)));
        }
        Ok(DecisionResult {
            decision: "imported",
            destination_path: Some(destination.display().to_string()),
            message: "Imported a new copy. The original file was not changed.".into(),
        })
    })
}

fn empty_item() -> ReviewItem {
    ReviewItem {
        state: "empty",
        candidate_id: None,
        relative_path: None,
        filename: None,
        media_type: None,
        effective_import_date: None,
        date_origin: None,
        tags: vec![],
        preview_url: None,
        message: "There are no pending items in this review.".into(),
    }
}
fn review_state(connection: &Connection) -> Result<ReviewState, ReviewError> {
    match connection.query_row("SELECT source_path, (SELECT count(*) FROM review_candidates WHERE session_id = review_sessions.id AND decision IS NULL) FROM review_sessions WHERE state = 'active' ORDER BY id DESC LIMIT 1", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))) { Ok((source_path, candidate_count)) => Ok(ReviewState { state: "resumable", source_path: Some(source_path), candidate_count, message: "Resume the safe read-only review. Originals are never modified.".into() }), Err(rusqlite::Error::QueryReturnedNoRows) => Ok(ReviewState { state: "none", source_path: None, candidate_count: 0, message: "No unfinished review is available.".into() }), Err(error) => Err(database_error(error)) }
}
fn pending_candidate(
    connection: &Connection,
    candidate_id: i64,
) -> Result<(String, String), ReviewError> {
    connection.query_row("SELECT s.source_path, c.relative_path FROM review_candidates c JOIN review_sessions s ON s.id = c.session_id WHERE c.id = ?1 AND s.state = 'active' AND c.decision IS NULL", [candidate_id], |row| Ok((row.get(0)?, row.get(1)?))).map_err(|value| match value { rusqlite::Error::QueryReturnedNoRows => error("candidate_not_pending", "This item has already been decided or is no longer available for review."), other => database_error(other) })
}
fn record_decision(
    connection: &Connection,
    candidate_id: i64,
    decision: &'static str,
    tags: &[String],
    destination: Option<&Path>,
    date: Option<&str>,
) -> Result<(), ReviewError> {
    pending_candidate(connection, candidate_id)?;
    let transaction = connection.unchecked_transaction().map_err(database_error)?;
    for tag in normalize_tags(tags) {
        transaction.execute("INSERT INTO tags (normalized_name) VALUES (?1) ON CONFLICT(normalized_name) DO NOTHING", [&tag]).map_err(database_error)?;
        transaction.execute("INSERT INTO candidate_tags (candidate_id, tag_id) SELECT ?1, id FROM tags WHERE normalized_name = ?2 ON CONFLICT(candidate_id, tag_id) DO NOTHING", params![candidate_id, tag]).map_err(database_error)?;
    }
    transaction
        .execute(
            "UPDATE review_candidates SET decision = ?1 WHERE id = ?2 AND decision IS NULL",
            params![decision, candidate_id],
        )
        .map_err(database_error)?;
    transaction.execute("INSERT INTO item_decisions (candidate_id, decision, destination_path, effective_import_date, date_origin) VALUES (?1, ?2, ?3, ?4, ?5)", params![candidate_id, decision, destination.map(|path| path.display().to_string()), date, if date.is_some() { Some("user") } else { None }]).map_err(database_error)?;
    transaction.commit().map_err(database_error)
}
fn candidate_tags(connection: &Connection, candidate_id: i64) -> Result<Vec<String>, ReviewError> {
    let mut statement = connection.prepare("SELECT t.normalized_name FROM tags t JOIN candidate_tags ct ON ct.tag_id = t.id WHERE ct.candidate_id = ?1 ORDER BY t.normalized_name").map_err(database_error)?;
    let tags = statement
        .query_map([candidate_id], |row| row.get(0))
        .map_err(database_error)?
        .collect::<Result<_, _>>()
        .map_err(database_error)?;
    Ok(tags)
}
fn normalize_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|tag| {
            tag.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        })
        .filter(|tag| !tag.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn canonical_directory(path: &str) -> Result<PathBuf, ReviewError> {
    let path = PathBuf::from(path.trim());
    if path.as_os_str().is_empty() {
        return Err(error("missing_source", "Choose an import folder first."));
    }
    if !fs::metadata(&path)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
    {
        return Err(error(
            "source_unavailable",
            "The selected import folder is unavailable.",
        ));
    }
    path.canonicalize().map_err(|_| {
        error(
            "source_unavailable",
            "The selected import folder is unavailable.",
        )
    })
}
fn discover(source: &Path) -> Result<Vec<Candidate>, ReviewError> {
    let mut candidates = Vec::new();
    visit(source, source, &mut candidates)?;
    candidates.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(candidates)
}
fn visit(
    root: &Path,
    directory: &Path,
    candidates: &mut Vec<Candidate>,
) -> Result<(), ReviewError> {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .map_err(|_| error("source_unavailable", "The import folder could not be read."))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| error("source_unavailable", "The import folder could not be read."))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|_| error("source_unavailable", "The import folder could not be read."))?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            if entry.file_name() != ".photo-handler" {
                visit(root, &path, candidates)?;
            }
        } else if kind.is_file() {
            if let Some(media_type) = media_type(&path) {
                let metadata = entry
                    .metadata()
                    .map_err(|_| error("source_unavailable", "A source file could not be read."))?;
                let modified_at = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs() as i64)
                    .unwrap_or(0);
                candidates.push(Candidate {
                    relative_path: path
                        .strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .into_owned(),
                    file_size: metadata.len() as i64,
                    modified_at,
                    media_type,
                });
            }
        }
    }
    Ok(())
}
fn media_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "heic" => Some("image"),
        "mp4" | "mov" | "m4v" | "webm" => Some("video"),
        _ => None,
    }
}
fn effective_date(path: &Path) -> Option<(String, String)> {
    metadata_date(path)
        .map(|date| (date, "metadata".into()))
        .or_else(|| {
            fs::metadata(path)
                .ok()?
                .created()
                .ok()
                .or_else(|| fs::metadata(path).ok()?.modified().ok())
                .map(|time| (format_date(time), "creation".into()))
        })
}
fn metadata_date(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut bytes = vec![0; 128 * 1024];
    let count = file.read(&mut bytes).ok()?;
    bytes.truncate(count);
    bytes.windows(19).find_map(|window| {
        if window[4] == b':'
            && window[7] == b':'
            && window[10] == b' '
            && window[13] == b':'
            && window[16] == b':'
            && window
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 4 | 7 | 10 | 13 | 16) || byte.is_ascii_digit())
        {
            Some(format!(
                "{}{}{}{}-{}{}-{}{}",
                window[0] as char,
                window[1] as char,
                window[2] as char,
                window[3] as char,
                window[5] as char,
                window[6] as char,
                window[8] as char,
                window[9] as char
            ))
        } else {
            None
        }
    })
}
fn format_date(time: SystemTime) -> String {
    DateTime::<Local>::from(time).format("%Y-%m-%d").to_string()
}
fn validate_date(value: &str) -> Result<String, ReviewError> {
    chrono::NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map(|date| date.format("%Y-%m-%d").to_string())
        .map_err(|_| {
            error(
                "invalid_import_date",
                "Use a valid import date in YYYY-MM-DD format.",
            )
        })
}
fn readable_file(path: &Path) -> Result<(), String> {
    if !fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        return Err("This source item is unavailable. It was not skipped or imported.".into());
    }
    fs::File::open(path)
        .map(|_| ())
        .map_err(|_| "This source item cannot be read. It was not skipped or imported.".into())
}

fn asset_url(path: &Path) -> Result<String, ReviewError> {
    let path = path.to_str().ok_or_else(|| {
        error(
            "preview_unavailable",
            "The selected item could not be prepared for in-app preview.",
        )
    })?;
    // Tauri's convertFileSrc encodes the whole absolute path, including slashes.
    // The asset handler then decodes it back to an absolute path before scope checks.
    Ok(format!(
        "asset://localhost/{}",
        utf8_percent_encode(path, NON_ALPHANUMERIC)
    ))
}
fn publish_copy(source: &Path, library: &Path, date: &str) -> Result<PathBuf, ReviewError> {
    readable_file(source).map_err(|message| error("candidate_unavailable", message))?;
    let year = date.get(0..4).expect("validated date has a year");
    let directory = library.join(year).join(date);
    fs::create_dir_all(&directory).map_err(|value| {
        error(
            "destination_unavailable",
            format!("Could not create the managed date folder: {value}"),
        )
    })?;
    let filename = source
        .file_name()
        .ok_or_else(|| {
            error(
                "candidate_unavailable",
                "The source item has no usable filename.",
            )
        })?
        .to_string_lossy()
        .into_owned();
    for attempt in 0..100 {
        let destination = unique_destination(&directory, &filename, attempt);
        if destination.exists() {
            continue;
        }
        let temporary = directory.join(format!(
            ".{filename}.photo-handler-pending-{}",
            random_suffix()
        ));
        let copy = (|| {
            let mut input = fs::File::open(source).map_err(|_| {
                error(
                    "candidate_unavailable",
                    "This source item cannot be read. It was not imported.",
                )
            })?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|_| {
                    error(
                        "destination_unavailable",
                        "Could not reserve a safe destination for the import.",
                    )
                })?;
            std::io::copy(&mut input, &mut output).map_err(|_| {
                error(
                    "copy_failed",
                    "The file copy did not finish. No import was published.",
                )
            })?;
            output.flush().map_err(|_| {
                error(
                    "copy_failed",
                    "The file copy did not finish. No import was published.",
                )
            })?;
            fs::hard_link(&temporary, &destination).map_err(|_| {
                error(
                    "destination_collision",
                    "A destination name collision occurred; retry the import to choose a new name.",
                )
            })?;
            fs::remove_file(&temporary).map_err(|_| {
                error(
                    "copy_failed",
                    "The copy was published but its temporary file could not be cleaned up safely.",
                )
            })?;
            Ok(destination)
        })();
        if copy.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        return copy;
    }
    Err(error(
        "destination_collision",
        "Could not reserve a unique destination name. No import was published.",
    ))
}
fn unique_destination(directory: &Path, filename: &str, attempt: u8) -> PathBuf {
    if attempt == 0 {
        return directory.join(filename);
    }
    let path = Path::new(filename);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = path
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    directory.join(format!("{stem}-{:02}{extension}", attempt + 1))
}
fn random_suffix() -> String {
    let mut bytes = [0_u8; 12];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}
fn error(code: &'static str, message: impl Into<String>) -> ReviewError {
    ReviewError {
        code,
        message: message.into(),
    }
}
fn database_error(value: rusqlite::Error) -> ReviewError {
    error(
        "catalogue_unavailable",
        format!("Could not update the protected review catalogue: {value}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn discovery_is_sorted_filters_media_and_never_mutates_source() {
        let source = tempdir().unwrap();
        fs::create_dir(source.path().join("nested")).unwrap();
        let photo = source.path().join("nested/a.JPG");
        fs::write(&photo, b"photo").unwrap();
        fs::write(source.path().join("z.txt"), b"ignore").unwrap();
        let before = fs::read(&photo).unwrap();
        let candidates = discover(source.path()).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].relative_path, "nested/a.JPG");
        assert_eq!(fs::read(&photo).unwrap(), before);
    }
    #[test]
    fn normalizes_tags_and_validates_dates() {
        assert_eq!(
            normalize_tags(&[
                " Beach ".into(),
                "beach".into(),
                "  Family  Trip ".into(),
                " ".into()
            ]),
            vec!["beach", "family trip"]
        );
        assert_eq!(validate_date("2024-02-29").unwrap(), "2024-02-29");
        assert_eq!(
            validate_date("2024-02-30").unwrap_err().code,
            "invalid_import_date"
        );
    }

    #[test]
    fn asset_urls_encode_the_entire_absolute_path() {
        let url = asset_url(Path::new("/Users/example/Pictures/family photo.jpg")).unwrap();
        assert_eq!(
            url,
            "asset://localhost/%2FUsers%2Fexample%2FPictures%2Ffamily%20photo%2Ejpg"
        );
    }
    #[test]
    fn copy_uses_date_folder_unique_name_and_preserves_source() {
        let source_dir = tempdir().unwrap();
        let library = tempdir().unwrap();
        let source = source_dir.path().join("family.jpg");
        fs::write(&source, b"original bytes").unwrap();
        let first = publish_copy(&source, library.path(), "2026-08-28").unwrap();
        let second = publish_copy(&source, library.path(), "2026-08-28").unwrap();
        assert_eq!(first.file_name().unwrap(), "family.jpg");
        assert_eq!(second.file_name().unwrap(), "family-02.jpg");
        assert_eq!(fs::read(first).unwrap(), b"original bytes");
        assert_eq!(fs::read(&source).unwrap(), b"original bytes");
    }
    #[test]
    fn unavailable_source_does_not_publish_or_change_it() {
        let source_dir = tempdir().unwrap();
        let library = tempdir().unwrap();
        let source = source_dir.path().join("gone.jpg");
        assert_eq!(
            publish_copy(&source, library.path(), "2026-08-28")
                .unwrap_err()
                .code,
            "candidate_unavailable"
        );
        assert!(fs::read_dir(library.path()).unwrap().next().is_none());
    }

    #[test]
    fn skip_and_normalized_tags_persist_after_reopening_the_catalogue() {
        let library_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        fs::write(source_dir.path().join("keep.jpg"), b"original bytes").unwrap();
        library::setup_library(library::SetupLibraryRequest {
            folder_path: library_dir.path().display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "First pet?".into(),
            recovery_answer: "Mochi".into(),
        })
        .unwrap();
        start_review(&source_dir.path().display().to_string()).unwrap();
        let candidate_id = library::with_catalogue(|connection, _| {
            connection
                .query_row("SELECT id FROM review_candidates", [], |row| row.get(0))
                .map_err(database_error)
        })
        .unwrap();
        skip_review_item(DecideRequest {
            candidate_id,
            tags: vec![" Family ".into(), "family".into()],
        })
        .unwrap();
        library::lock_library();
        library::unlock_library(
            &library_dir.path().display().to_string(),
            library::UnlockLibraryRequest {
                password: "correct horse battery staple".into(),
            },
        )
        .unwrap();
        let (decision, tag): (String, String) = library::with_catalogue(|connection, _| {
            connection
                .query_row(
                    "SELECT d.decision, t.normalized_name FROM item_decisions d JOIN candidate_tags ct ON ct.candidate_id = d.candidate_id JOIN tags t ON t.id = ct.tag_id",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(database_error)
        })
        .unwrap();
        assert_eq!(decision, "skipped");
        assert_eq!(tag, "family");
        assert_eq!(
            fs::read(source_dir.path().join("keep.jpg")).unwrap(),
            b"original bytes"
        );
    }
    #[test]
    fn review_commands_require_an_unlocked_library_session() {
        library::lock_library();
        assert_eq!(current_review_state().unwrap_err().code, "library_locked");
    }
}

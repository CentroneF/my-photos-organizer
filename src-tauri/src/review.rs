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
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::library::{self, SetupLibraryError};
use crate::search;

const CONTENT_FINGERPRINT_ALGORITHM: &str = "blake3-256-v1";
const EXACT_HISTORY_LIMIT: usize = 3;
const PERCEPTUAL_HASH_ALGORITHM: &str = "dhash-64-v1";
const SIMILARITY_LIMIT: usize = 3;
const SIMILARITY_THRESHOLD: u32 = 10;
const MAX_DECODED_PIXELS: u64 = 40_000_000;

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
    pub exact_matches: Vec<ExactMatch>,
    pub similar_matches: Vec<SimilarMatch>,
    pub visual_comparison_message: Option<String>,
    pub imported_count: u64,
    pub skipped_count: u64,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactMatch {
    pub decision: String,
    pub filename: String,
    pub relative_path: String,
    pub decided_at: String,
    pub tags: Vec<String>,
    pub preview_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarMatch {
    pub filename: String,
    pub decided_at: String,
    pub tags: Vec<String>,
    pub preview_url: Option<String>,
    pub similarity_label: &'static str,
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
        let candidates = discover(&source)?;
        if let Some((session_id, session_state)) = connection
            .query_row(
                "SELECT id, state FROM review_sessions WHERE source_path = ?1",
                [&source_text],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(database_error)?
        {
            let transaction = connection.unchecked_transaction().map_err(database_error)?;
            let mut additions = 0;
            for candidate in &candidates {
                let latest = transaction.query_row(
                    "SELECT file_size, modified_at, media_type, revision, content_fingerprint_value FROM review_candidates WHERE session_id = ?1 AND relative_path = ?2 ORDER BY revision DESC LIMIT 1",
                    params![session_id, candidate.relative_path],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?, row.get::<_, Option<Vec<u8>>>(4)?)),
                ).optional().map_err(database_error)?;
                let revision = match latest {
                    Some((size, modified, media_type, _revision, fingerprint))
                        if size == candidate.file_size
                            && modified == candidate.modified_at
                            && media_type == candidate.media_type =>
                    {
                        let unchanged = fingerprint
                            .map(|fingerprint| {
                                fingerprint_file(
                                    &source.join(&candidate.relative_path),
                                    candidate.file_size,
                                    candidate.modified_at,
                                )
                                .map(|current| current == fingerprint)
                                .unwrap_or(false)
                            })
                            .unwrap_or(true);
                        if unchanged {
                            continue;
                        }
                        _revision + 1
                    }
                    Some((_, _, _, revision, _)) => revision + 1,
                    None => 1,
                };
                additions += transaction.execute(
                    "INSERT INTO review_candidates (session_id, relative_path, revision, file_size, modified_at, media_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![session_id, candidate.relative_path, revision, candidate.file_size, candidate.modified_at, candidate.media_type],
                ).map_err(database_error)?;
            }
            if additions > 0 {
                transaction
                    .execute(
                        "UPDATE review_sessions SET state = 'active' WHERE id = ?1",
                        [session_id],
                    )
                    .map_err(database_error)?;
            }
            let pending_count = transaction
                .query_row(
                    "SELECT count(*) FROM review_candidates WHERE session_id = ?1 AND decision IS NULL",
                    [session_id],
                    |row| row.get::<_, u64>(0),
                )
                .map_err(database_error)?;
            transaction.commit().map_err(database_error)?;
            return Ok(ReviewState {
                state: if pending_count > 0 {
                    "resumable"
                } else {
                    "complete"
                },
                source_path: Some(source_text),
                candidate_count: pending_count,
                message: if additions > 0 {
                    format!("Found {additions} newly added supported item(s). Previous decisions remain unchanged; originals are never modified.")
                } else if session_state == "complete" {
                    "This source review is complete. No new supported files were found; originals were not deleted or moved.".into()
                } else {
                    "Resuming the existing review session. Originals are never modified.".into()
                },
            });
        }
        let transaction = connection.unchecked_transaction().map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO review_sessions (source_path, state) VALUES (?1, 'active')",
                [&source_text],
            )
            .map_err(database_error)?;
        let session_id = transaction.last_insert_rowid();
        for candidate in &candidates {
            transaction.execute("INSERT INTO review_candidates (session_id, relative_path, revision, file_size, modified_at, media_type) VALUES (?1, ?2, 1, ?3, ?4, ?5)", params![session_id, candidate.relative_path, candidate.file_size, candidate.modified_at, candidate.media_type]).map_err(database_error)?;
        }
        if candidates.is_empty() {
            transaction
                .execute(
                    "UPDATE review_sessions SET state = 'complete' WHERE id = ?1",
                    [session_id],
                )
                .map_err(database_error)?;
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
    library::with_catalogue(|connection, library_path| {
        let (session_id, session_state) = match connection.query_row(
            "SELECT id, state FROM review_sessions ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        ) {
            Ok(session) => session,
            Err(rusqlite::Error::QueryReturnedNoRows) => return completion_item(connection),
            Err(value) => return Err(database_error(value)),
        };
        if session_state == "complete" {
            return completion_item_for_session(connection, session_id);
        }
        let row = connection.query_row("SELECT c.id, s.source_path, c.relative_path, c.media_type, c.file_size, c.modified_at FROM review_candidates c JOIN review_sessions s ON s.id = c.session_id WHERE c.session_id = ?1 AND c.decision IS NULL ORDER BY c.relative_path, c.revision LIMIT 1", [session_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4)?, row.get::<_, i64>(5)?)));
        let (candidate_id, source_path, relative_path, media_type, file_size, modified_at) =
            match row {
                Ok(row) => row,
                Err(rusqlite::Error::QueryReturnedNoRows) => return completion_item(connection),
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
        let fingerprint = match fingerprint_file(&path, file_size, modified_at) {
            Ok(fingerprint) => fingerprint,
            Err(message) => {
                return Ok(unavailable_item(
                    candidate_id,
                    relative_path,
                    filename,
                    media_type,
                    date,
                    origin,
                    tags,
                    message,
                ))
            }
        };
        let visual_result = perceptual_hash(&path);
        if stable_metadata(&path) != Ok((file_size, modified_at)) {
            return Ok(unavailable_item(
                candidate_id,
                relative_path,
                filename,
                media_type,
                date,
                origin,
                tags,
                "This source item changed while it was being compared. Refresh the review before deciding; no fingerprint was saved.".into(),
            ));
        }
        if connection.execute(
            "UPDATE review_candidates SET content_fingerprint_algorithm = ?1, content_fingerprint_value = ?2 WHERE id = ?3",
            params![CONTENT_FINGERPRINT_ALGORITHM, fingerprint, candidate_id],
        ).is_err() {
            return Ok(unavailable_item(candidate_id, relative_path, filename, media_type, date, origin, tags, "Comparison results could not be saved. Refresh the review and try again; no decision was made.".into()));
        }
        let exact_matches = match exact_matches(connection, &app, library_path, candidate_id, &fingerprint) {
            Ok(matches) => matches,
            Err(_) => return Ok(unavailable_item(candidate_id, relative_path, filename, media_type, date, origin, tags, "Comparison history is temporarily unavailable. Refresh the review and try again; no decision was made.".into())),
        };
        let (similar_matches, visual_comparison_message) = match visual_result {
            Ok(hash) => {
                if connection.execute("UPDATE review_candidates SET perceptual_hash_algorithm = ?1, perceptual_hash_value = ?2, perceptual_hash_threshold = ?3, visual_comparison_state = 'available' WHERE id = ?4", params![PERCEPTUAL_HASH_ALGORITHM, hash as i64, SIMILARITY_THRESHOLD as i64, candidate_id]).is_err() {
                    return Ok(unavailable_item(candidate_id, relative_path, filename, media_type, date, origin, tags, "Comparison results could not be saved. Refresh the review and try again; no decision was made.".into()));
                }
                let matches = match similar_matches(connection, &app, library_path, candidate_id, hash) {
                    Ok(matches) => matches,
                    Err(_) => return Ok(unavailable_item(candidate_id, relative_path, filename, media_type, date, origin, tags, "Comparison history is temporarily unavailable. Refresh the review and try again; no decision was made.".into())),
                };
                (matches, None)
            }
            Err(message) => {
                if connection
                    .execute(
                        "UPDATE review_candidates SET visual_comparison_state = ?1 WHERE id = ?2",
                        params![message.0, candidate_id],
                    )
                    .is_err()
                {
                    return Ok(unavailable_item(candidate_id, relative_path, filename, media_type, date, origin, tags, "Comparison results could not be saved. Refresh the review and try again; no decision was made.".into()));
                }
                (vec![], Some(message.1))
            }
        };
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
                return Ok(unavailable_item(
                    candidate_id,
                    relative_path,
                    filename,
                    media_type,
                    date,
                    origin,
                    tags,
                    message,
                ))
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
            exact_matches,
            similar_matches,
            visual_comparison_message,
            imported_count: 0,
            skipped_count: 0,
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
        let original_date = effective_date(&source);
        let destination = publish_copy(&source, library_path, &date)?;
        if let Err(record_error) = record_decision(
            connection,
            request.candidate_id,
            "imported",
            &request.tags,
            Some(&destination),
            Some(&date),
            original_date
                .as_ref()
                .map(|(date, origin)| (date.as_str(), origin.as_str())),
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

fn completion_item(connection: &Connection) -> Result<ReviewItem, ReviewError> {
    let session_id = connection
        .query_row(
            "SELECT id FROM review_sessions WHERE state = 'complete' ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    match session_id {
        Some(id) => completion_item_for_session(connection, id),
        None => Ok(empty_item()),
    }
}

fn completion_item_for_session(
    connection: &Connection,
    session_id: i64,
) -> Result<ReviewItem, ReviewError> {
    let result = connection.query_row(
        "SELECT s.source_path, \
            (SELECT count(*) FROM review_candidates WHERE session_id = s.id AND decision = 'imported'), \
            (SELECT count(*) FROM review_candidates WHERE session_id = s.id AND decision = 'skipped') \
         FROM review_sessions s WHERE s.id = ?1 AND s.state = 'complete'",
        [session_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?, row.get::<_, u64>(2)?)),
    );
    match result {
        Ok((source_path, imported_count, skipped_count)) => Ok(ReviewItem {
            state: "complete",
            candidate_id: None,
            relative_path: None,
            filename: None,
            media_type: None,
            effective_import_date: None,
            date_origin: None,
            tags: vec![],
            preview_url: None,
            exact_matches: vec![],
            similar_matches: vec![],
            visual_comparison_message: None,
            imported_count,
            skipped_count,
            message: format!(
                "Review complete for {source_path}. Originals were not deleted, moved, or changed."
            ),
        }),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(empty_item()),
        Err(value) => Err(database_error(value)),
    }
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
        exact_matches: vec![],
        similar_matches: vec![],
        visual_comparison_message: None,
        imported_count: 0,
        skipped_count: 0,
        message: "There are no pending items in this review.".into(),
    }
}

fn unavailable_item(
    candidate_id: i64,
    relative_path: String,
    filename: String,
    media_type: String,
    date: String,
    origin: String,
    tags: Vec<String>,
    message: String,
) -> ReviewItem {
    ReviewItem {
        state: "unavailable",
        candidate_id: Some(candidate_id),
        relative_path: Some(relative_path),
        filename: Some(filename),
        media_type: Some(media_type),
        effective_import_date: Some(date),
        date_origin: Some(origin),
        tags,
        preview_url: None,
        exact_matches: vec![],
        similar_matches: vec![],
        visual_comparison_message: None,
        imported_count: 0,
        skipped_count: 0,
        message,
    }
}

fn fingerprint_file(
    path: &Path,
    expected_size: i64,
    expected_modified_at: i64,
) -> Result<Vec<u8>, String> {
    fingerprint_file_with_after_read(path, expected_size, expected_modified_at, || {})
}

fn fingerprint_file_with_after_read(
    path: &Path,
    expected_size: i64,
    expected_modified_at: i64,
    after_read: impl FnOnce(),
) -> Result<Vec<u8>, String> {
    let before = stable_metadata(path)?;
    if before != (expected_size, expected_modified_at) {
        return Err("This source item changed since discovery. Refresh the review before deciding; no fingerprint was saved.".into());
    }
    let mut file = fs::File::open(path).map_err(|_| {
        "This source item cannot be read. It was not skipped or imported.".to_owned()
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|_| {
            "This source item cannot be read. It was not skipped or imported.".to_owned()
        })?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    after_read();
    if stable_metadata(path)? != before {
        return Err("This source item changed while it was being checked. Refresh the review before deciding; no fingerprint was saved.".into());
    }
    Ok(hasher.finalize().as_bytes().to_vec())
}

fn stable_metadata(path: &Path) -> Result<(i64, i64), String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        "This source item is unavailable. It was not skipped or imported.".to_owned()
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err("This source item is unavailable. It was not skipped or imported.".into());
    }
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    Ok((metadata.len() as i64, modified))
}

fn exact_matches(
    connection: &Connection,
    app: &tauri::AppHandle,
    library_path: &Path,
    active_candidate_id: i64,
    fingerprint: &[u8],
) -> Result<Vec<ExactMatch>, ReviewError> {
    let mut statement = connection.prepare(
        "SELECT c.id, c.relative_path, d.decision, d.decided_at, d.destination_path \
         FROM review_candidates c JOIN item_decisions d ON d.candidate_id = c.id \
         WHERE c.content_fingerprint_algorithm = ?1 AND c.content_fingerprint_value = ?2 AND c.id != ?3 \
         ORDER BY d.decided_at DESC, c.id LIMIT ?4"
    ).map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                CONTENT_FINGERPRINT_ALGORITHM,
                fingerprint,
                active_candidate_id,
                EXACT_HISTORY_LIMIT as i64
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .map_err(database_error)?;
    rows.map(|row| {
        let (candidate_id, relative_path, decision, decided_at, destination) =
            row.map_err(database_error)?;
        let preview_url = if decision == "imported" {
            destination.and_then(|destination| {
                search::safe_preview_url(app, library_path, Path::new(&destination)).0
            })
        } else {
            None
        };
        Ok(ExactMatch {
            decision,
            filename: Path::new(&relative_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            relative_path,
            decided_at,
            tags: candidate_tags(connection, candidate_id)?,
            preview_url,
        })
    })
    .collect()
}

fn perceptual_hash(path: &Path) -> Result<u64, (&'static str, String)> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif") {
        return Err((
            "unsupported",
            "Visual comparison is unavailable for this media type; exact history is still checked."
                .into(),
        ));
    }
    let reader = image::ImageReader::open(path).map_err(|_| {
        (
            "decode_failed",
            "Visual comparison is unavailable because this image could not be read.".into(),
        )
    })?;
    let reader = reader.with_guessed_format().map_err(|_| {
        (
            "decode_failed",
            "Visual comparison is unavailable because this image format could not be read.".into(),
        )
    })?;
    let dimensions = reader.into_dimensions().map_err(|_| {
        (
            "decode_failed",
            "Visual comparison is unavailable because this image could not be decoded.".into(),
        )
    })?;
    if u64::from(dimensions.0) * u64::from(dimensions.1) > MAX_DECODED_PIXELS {
        return Err((
            "resource_limited",
            "Visual comparison is unavailable because this image is too large to compare safely."
                .into(),
        ));
    }
    let image = image::open(path).map_err(|_| {
        (
            "decode_failed",
            "Visual comparison is unavailable because this image could not be decoded.".into(),
        )
    })?;
    let pixels = image
        .resize_exact(9, 8, image::imageops::FilterType::Triangle)
        .to_luma8();
    let mut hash = 0_u64;
    for y in 0..8 {
        for x in 0..8 {
            hash =
                (hash << 1) | u64::from(pixels.get_pixel(x, y)[0] > pixels.get_pixel(x + 1, y)[0]);
        }
    }
    Ok(hash)
}

fn similar_matches(
    connection: &Connection,
    app: &tauri::AppHandle,
    root: &Path,
    active_id: i64,
    hash: u64,
) -> Result<Vec<SimilarMatch>, ReviewError> {
    let mut statement = connection.prepare("SELECT c.id, c.relative_path, d.decided_at, d.destination_path, c.perceptual_hash_value FROM review_candidates c JOIN item_decisions d ON d.candidate_id = c.id WHERE d.decision = 'imported' AND d.destination_path IS NOT NULL AND c.perceptual_hash_algorithm = ?1 AND c.perceptual_hash_threshold = ?2 AND c.id != ?3 AND c.content_fingerprint_value != (SELECT content_fingerprint_value FROM review_candidates WHERE id = ?3) ORDER BY d.decided_at DESC, c.id").map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                PERCEPTUAL_HASH_ALGORITHM,
                SIMILARITY_THRESHOLD as i64,
                active_id
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .map_err(database_error)?;
    let mut candidates = Vec::new();
    for row in rows {
        let (id, relative_path, decided_at, destination, candidate_hash) =
            row.map_err(database_error)?;
        let distance = (hash ^ candidate_hash as u64).count_ones();
        if distance > SIMILARITY_THRESHOLD {
            continue;
        }
        candidates.push((distance, id, relative_path, decided_at, destination));
    }
    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| right.3.cmp(&left.3))
            .then_with(|| left.1.cmp(&right.1))
    });
    Ok(candidates
        .into_iter()
        .take(SIMILARITY_LIMIT)
        .map(|(_, id, relative_path, decided_at, destination)| {
            Ok::<_, ReviewError>(SimilarMatch {
                filename: Path::new(&relative_path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                decided_at,
                tags: candidate_tags(connection, id)?,
                preview_url: search::safe_preview_url(app, root, Path::new(&destination)).0,
                similarity_label: "Possible similar picture",
            })
        })
        .collect::<Result<Vec<_>, _>>()?)
}
fn review_state(connection: &Connection) -> Result<ReviewState, ReviewError> {
    match connection.query_row("SELECT source_path, state, (SELECT count(*) FROM review_candidates WHERE session_id = review_sessions.id AND decision IS NULL) FROM review_sessions ORDER BY id DESC LIMIT 1", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, u64>(2)?))) { Ok((source_path, state, candidate_count)) => Ok(ReviewState { state: if state == "complete" { "complete" } else { "resumable" }, source_path: Some(source_path), candidate_count, message: if state == "complete" { "Resume to check this source for newly added supported files. Originals are never modified.".into() } else { "Resume the safe read-only review. Originals are never modified.".into() } }), Err(rusqlite::Error::QueryReturnedNoRows) => Ok(ReviewState { state: "none", source_path: None, candidate_count: 0, message: "No unfinished review is available.".into() }), Err(error) => Err(database_error(error)) }
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
    original_date: Option<(&str, &str)>,
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
    transaction.execute("INSERT INTO item_decisions (candidate_id, decision, destination_path, effective_import_date, date_origin, original_media_date, original_date_origin) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![candidate_id, decision, destination.map(|path| path.display().to_string()), date, if date.is_some() { Some("user") } else { None }, original_date.map(|(date, _)| date), original_date.map(|(_, origin)| origin)]).map_err(database_error)?;
    transaction
        .execute(
            "UPDATE review_sessions SET state = 'complete' WHERE id = (SELECT session_id FROM review_candidates WHERE id = ?1) AND NOT EXISTS (SELECT 1 FROM review_candidates WHERE session_id = (SELECT session_id FROM review_candidates WHERE id = ?1) AND decision IS NULL)",
            [candidate_id],
        )
        .map_err(database_error)?;
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
        let _session_guard = library::test_session_guard();
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
        let _session_guard = library::test_session_guard();
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
        let _session_guard = library::test_session_guard();
        let url = asset_url(Path::new("/Users/example/Pictures/family photo.jpg")).unwrap();
        assert_eq!(
            url,
            "asset://localhost/%2FUsers%2Fexample%2FPictures%2Ffamily%20photo%2Ejpg"
        );
    }
    #[test]
    fn copy_uses_date_folder_unique_name_and_preserves_source() {
        let _session_guard = library::test_session_guard();
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
        let _session_guard = library::test_session_guard();
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
    fn fingerprints_equal_bytes_but_rejects_equal_size_different_bytes() {
        let _session_guard = library::test_session_guard();
        let directory = tempdir().unwrap();
        let first = directory.path().join("first.jpg");
        let second = directory.path().join("second.jpg");
        let different = directory.path().join("third.jpg");
        fs::write(&first, b"same bytes").unwrap();
        fs::write(&second, b"same bytes").unwrap();
        fs::write(&different, b"same bytez").unwrap();
        let first_metadata = stable_metadata(&first).unwrap();
        let second_metadata = stable_metadata(&second).unwrap();
        let different_metadata = stable_metadata(&different).unwrap();
        assert_eq!(
            fingerprint_file(&first, first_metadata.0, first_metadata.1).unwrap(),
            fingerprint_file(&second, second_metadata.0, second_metadata.1).unwrap()
        );
        assert_ne!(
            fingerprint_file(&first, first_metadata.0, first_metadata.1).unwrap(),
            fingerprint_file(&different, different_metadata.0, different_metadata.1).unwrap()
        );
        assert_eq!(fs::read(&first).unwrap(), b"same bytes");
    }

    #[test]
    fn changed_during_hash_returns_recoverable_feedback_without_a_digest() {
        let _session_guard = library::test_session_guard();
        let directory = tempdir().unwrap();
        let source = directory.path().join("changing.jpg");
        fs::write(&source, b"first source bytes").unwrap();
        let (size, modified_at) = stable_metadata(&source).unwrap();

        let error = fingerprint_file_with_after_read(&source, size, modified_at, || {
            fs::write(&source, b"replacement source bytes with a new size").unwrap();
        })
        .unwrap_err();

        assert!(error.contains("changed while it was being checked"));
        assert_eq!(
            fs::read(&source).unwrap(),
            b"replacement source bytes with a new size"
        );
    }

    #[test]
    fn dhash_calibration_accepts_a_small_brightness_change_but_rejects_an_unrelated_image() {
        let _session_guard = library::test_session_guard();
        let directory = tempdir().unwrap();
        let base = directory.path().join("base.png");
        let brighter = directory.path().join("brighter.png");
        let unrelated = directory.path().join("unrelated.png");
        let base_image = image::RgbImage::from_fn(48, 48, |x, y| {
            image::Rgb([(x * 5) as u8, (y * 5) as u8, ((x + y) * 2) as u8])
        });
        let brighter_image = image::RgbImage::from_fn(48, 48, |x, y| {
            image::Rgb([
                ((x * 5) + 10) as u8,
                ((y * 5) + 10) as u8,
                (((x + y) * 2) + 10) as u8,
            ])
        });
        let unrelated_image = image::RgbImage::from_fn(48, 48, |x, y| {
            image::Rgb([
                if (x / 6 + y / 6) % 2 == 0 { 0 } else { 255 },
                255 - (x * 5) as u8,
                0,
            ])
        });
        base_image.save(&base).unwrap();
        brighter_image.save(&brighter).unwrap();
        unrelated_image.save(&unrelated).unwrap();
        let base_hash = perceptual_hash(&base).unwrap();
        assert!(
            (base_hash ^ perceptual_hash(&brighter).unwrap()).count_ones() <= SIMILARITY_THRESHOLD
        );
        assert!(
            (base_hash ^ perceptual_hash(&unrelated).unwrap()).count_ones() > SIMILARITY_THRESHOLD
        );
    }

    #[test]
    fn visual_comparison_reports_unsupported_and_decode_failure() {
        let _session_guard = library::test_session_guard();
        let directory = tempdir().unwrap();
        let video = directory.path().join("clip.mp4");
        let corrupt = directory.path().join("broken.png");
        fs::write(&video, b"video").unwrap();
        fs::write(&corrupt, b"not an image").unwrap();
        assert_eq!(perceptual_hash(&video).unwrap_err().0, "unsupported");
        assert_eq!(perceptual_hash(&corrupt).unwrap_err().0, "decode_failed");
    }

    #[test]
    fn resume_appends_a_revision_when_a_decided_path_changes() {
        let _session_guard = library::test_session_guard();
        let library_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        let source = source_dir.path().join("same.jpg");
        fs::write(&source, b"first original").unwrap();
        library::setup_library(library::SetupLibraryRequest {
            folder_path: library_dir.path().display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "First pet?".into(),
            recovery_answer: "Mochi".into(),
        })
        .unwrap();
        start_review(&source_dir.path().display().to_string()).unwrap();
        let first_id = library::with_catalogue(|connection, _| {
            connection
                .query_row("SELECT id FROM review_candidates", [], |row| row.get(0))
                .map_err(database_error)
        })
        .unwrap();
        skip_review_item(DecideRequest {
            candidate_id: first_id,
            tags: vec![],
        })
        .unwrap();
        fs::write(&source, b"second original with different size").unwrap();
        let resumed = start_review(&source_dir.path().display().to_string()).unwrap();
        assert_eq!(resumed.candidate_count, 1);
        let rows: Vec<(i64, Option<String>)> = library::with_catalogue(|connection, _| {
            let mut statement = connection.prepare("SELECT revision, decision FROM review_candidates WHERE relative_path = 'same.jpg' ORDER BY revision").map_err(database_error)?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(database_error)?.collect::<Result<_, _>>().map_err(database_error);
            rows
        }).unwrap();
        assert_eq!(rows, vec![(1, Some("skipped".into())), (2, None)]);
        assert_eq!(
            fs::read(&source).unwrap(),
            b"second original with different size"
        );
    }

    #[test]
    fn resume_appends_a_revision_when_same_metadata_has_different_bytes() {
        let _session_guard = library::test_session_guard();
        let library_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        let source = source_dir.path().join("same.jpg");
        fs::write(&source, b"first-original").unwrap();
        let original_metadata = stable_metadata(&source).unwrap();
        let original_fingerprint =
            fingerprint_file(&source, original_metadata.0, original_metadata.1).unwrap();
        library::setup_library(library::SetupLibraryRequest {
            folder_path: library_dir.path().display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "First pet?".into(),
            recovery_answer: "Mochi".into(),
        })
        .unwrap();
        start_review(&source_dir.path().display().to_string()).unwrap();
        let first_id = library::with_catalogue(|connection, _| {
            connection
                .query_row("SELECT id FROM review_candidates", [], |row| row.get(0))
                .map_err(database_error)
        })
        .unwrap();
        skip_review_item(DecideRequest {
            candidate_id: first_id,
            tags: vec![],
        })
        .unwrap();
        fs::write(&source, b"second-originl").unwrap();
        let replacement_metadata = stable_metadata(&source).unwrap();
        assert_eq!(replacement_metadata.0, original_metadata.0);
        library::with_catalogue(|connection, _| {
            connection
                .execute(
                    "UPDATE review_candidates SET modified_at = ?1, content_fingerprint_algorithm = ?2, content_fingerprint_value = ?3 WHERE id = ?4",
                    params![replacement_metadata.1, CONTENT_FINGERPRINT_ALGORITHM, original_fingerprint, first_id],
                )
                .map_err(database_error)?;
            Ok::<_, ReviewError>(())
        })
        .unwrap();

        let resumed = start_review(&source_dir.path().display().to_string()).unwrap();
        assert_eq!(resumed.candidate_count, 1);
        let rows: Vec<(i64, Option<String>)> = library::with_catalogue(|connection, _| {
            let mut statement = connection
                .prepare("SELECT revision, decision FROM review_candidates WHERE relative_path = 'same.jpg' ORDER BY revision")
                .map_err(database_error)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(database_error)?
                .collect::<Result<_, _>>()
                .map_err(database_error);
            rows
        })
        .unwrap();
        assert_eq!(rows, vec![(1, Some("skipped".into())), (2, None)]);
    }

    #[test]
    fn exact_history_query_is_capped_and_orders_newest_decisions_first() {
        let _session_guard = library::test_session_guard();
        let library_dir = tempdir().unwrap();
        library::setup_library(library::SetupLibraryRequest {
            folder_path: library_dir.path().display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "First pet?".into(),
            recovery_answer: "Mochi".into(),
        })
        .unwrap();
        let digest = vec![7_u8; 32];
        let ids = library::with_catalogue(|connection, _| {
            connection.execute_batch("INSERT INTO review_sessions (id, source_path, state) VALUES (1, 'source', 'complete');").map_err(database_error)?;
            for id in 1..=4 {
                connection.execute("INSERT INTO review_candidates (id, session_id, relative_path, revision, file_size, modified_at, media_type, content_fingerprint_algorithm, content_fingerprint_value, decision) VALUES (?1, 1, ?2, 1, 1, 1, 'image', ?3, ?4, 'skipped')", params![id, format!("{id}.jpg"), CONTENT_FINGERPRINT_ALGORITHM, digest]).map_err(database_error)?;
                connection.execute("INSERT INTO item_decisions (candidate_id, decision, decided_at) VALUES (?1, 'skipped', ?2)", params![id, format!("2026-08-0{id} 00:00:00")]).map_err(database_error)?;
            }
            let mut statement = connection.prepare("SELECT c.id FROM review_candidates c JOIN item_decisions d ON d.candidate_id = c.id WHERE c.content_fingerprint_algorithm = ?1 AND c.content_fingerprint_value = ?2 ORDER BY d.decided_at DESC, c.id LIMIT ?3").map_err(database_error)?;
            let ids = statement.query_map(params![CONTENT_FINGERPRINT_ALGORITHM, digest, EXACT_HISTORY_LIMIT as i64], |row| row.get(0)).map_err(database_error)?.collect::<Result<Vec<i64>, _>>().map_err(database_error)?;
            Ok::<_, ReviewError>(ids)
        }).unwrap();
        assert_eq!(ids, vec![4, 3, 2]);
    }

    #[test]
    fn skip_and_normalized_tags_persist_after_reopening_the_catalogue() {
        let _session_guard = library::test_session_guard();
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
    fn importing_preserves_the_original_discovered_date_across_reopen() {
        let _session_guard = library::test_session_guard();
        let library_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        fs::write(
            source_dir.path().join("dated.jpg"),
            b"Exif 2020:07:14 10:20:30",
        )
        .unwrap();
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
        import_review_item(ImportRequest {
            candidate_id,
            tags: vec![],
            effective_import_date: "2026-08-28".into(),
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
        let dates = library::with_catalogue(|connection, _| {
            connection
                .query_row(
                    "SELECT effective_import_date, original_media_date, original_date_origin FROM item_decisions",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?)),
                )
                .map_err(database_error)
        })
        .unwrap();
        assert_eq!(dates.0, "2026-08-28");
        assert_eq!(dates.1.as_deref(), Some("2020-07-14"));
        assert_eq!(dates.2.as_deref(), Some("metadata"));
        library::lock_library();
    }
    #[test]
    fn review_commands_require_an_unlocked_library_session() {
        let _session_guard = library::test_session_guard();
        library::lock_library();
        assert_eq!(current_review_state().unwrap_err().code, "library_locked");
    }

    #[test]
    fn completing_a_review_persists_counts_across_lock_and_reopen() {
        let _session_guard = library::test_session_guard();
        let library_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        fs::write(source_dir.path().join("import.jpg"), b"import me").unwrap();
        fs::write(source_dir.path().join("skip.jpg"), b"skip me").unwrap();
        library::setup_library(library::SetupLibraryRequest {
            folder_path: library_dir.path().display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "First pet?".into(),
            recovery_answer: "Mochi".into(),
        })
        .unwrap();
        start_review(&source_dir.path().display().to_string()).unwrap();
        let candidate_ids = library::with_catalogue(|connection, _| {
            let mut statement = connection
                .prepare("SELECT id FROM review_candidates ORDER BY relative_path")
                .map_err(database_error)?;
            let candidate_ids = statement
                .query_map([], |row| row.get::<_, i64>(0))
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            Ok::<_, ReviewError>(candidate_ids)
        })
        .unwrap();
        import_review_item(ImportRequest {
            candidate_id: candidate_ids[0],
            tags: vec!["kept".into()],
            effective_import_date: "2026-08-28".into(),
        })
        .unwrap();
        skip_review_item(DecideRequest {
            candidate_id: candidate_ids[1],
            tags: vec![],
        })
        .unwrap();

        library::lock_library();
        assert_eq!(current_review_state().unwrap_err().code, "library_locked");
        library::unlock_library(
            &library_dir.path().display().to_string(),
            library::UnlockLibraryRequest {
                password: "correct horse battery staple".into(),
            },
        )
        .unwrap();
        let completed =
            library::with_catalogue(|connection, _| completion_item(connection)).unwrap();
        assert_eq!(completed.state, "complete");
        assert_eq!(completed.imported_count, 1);
        assert_eq!(completed.skipped_count, 1);
        assert_eq!(
            fs::read(source_dir.path().join("import.jpg")).unwrap(),
            b"import me"
        );
        assert_eq!(
            fs::read(source_dir.path().join("skip.jpg")).unwrap(),
            b"skip me"
        );
    }

    #[test]
    fn resuming_a_completed_source_adds_only_new_candidates() {
        let _session_guard = library::test_session_guard();
        let library_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        let first = source_dir.path().join("already-reviewed.jpg");
        fs::write(&first, b"first original").unwrap();
        library::setup_library(library::SetupLibraryRequest {
            folder_path: library_dir.path().display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "First pet?".into(),
            recovery_answer: "Mochi".into(),
        })
        .unwrap();
        start_review(&source_dir.path().display().to_string()).unwrap();
        let first_candidate = library::with_catalogue(|connection, _| {
            connection
                .query_row("SELECT id FROM review_candidates", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(database_error)
        })
        .unwrap();
        skip_review_item(DecideRequest {
            candidate_id: first_candidate,
            tags: vec!["reviewed".into()],
        })
        .unwrap();

        let new_file = source_dir.path().join("nested/newly-added.mp4");
        fs::create_dir(new_file.parent().unwrap()).unwrap();
        fs::write(&new_file, b"new original").unwrap();
        let resumed = start_review(&source_dir.path().display().to_string()).unwrap();
        assert_eq!(resumed.state, "resumable");
        assert_eq!(resumed.candidate_count, 1);
        let candidates = library::with_catalogue(|connection, _| {
            let mut statement = connection
                .prepare(
                    "SELECT relative_path, decision FROM review_candidates ORDER BY relative_path",
                )
                .map_err(database_error)?;
            let candidates = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                })
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            Ok::<_, ReviewError>(candidates)
        })
        .unwrap();
        assert_eq!(
            candidates,
            vec![
                ("already-reviewed.jpg".into(), Some("skipped".into())),
                ("nested/newly-added.mp4".into(), None),
            ]
        );
        let new_candidate = library::with_catalogue(|connection, _| {
            connection
                .query_row(
                    "SELECT id FROM review_candidates WHERE relative_path = 'nested/newly-added.mp4'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)
        })
        .unwrap();
        skip_review_item(DecideRequest {
            candidate_id: new_candidate,
            tags: vec![],
        })
        .unwrap();
        let completion =
            library::with_catalogue(|connection, _| completion_item(connection)).unwrap();
        assert_eq!(completion.skipped_count, 2);
        assert_eq!(fs::read(first).unwrap(), b"first original");
        assert_eq!(fs::read(new_file).unwrap(), b"new original");
    }

    #[test]
    fn failed_decision_does_not_complete_or_change_the_source() {
        let _session_guard = library::test_session_guard();
        let library_dir = tempdir().unwrap();
        let source_dir = tempdir().unwrap();
        let source = source_dir.path().join("still-pending.jpg");
        fs::write(&source, b"original").unwrap();
        library::setup_library(library::SetupLibraryRequest {
            folder_path: library_dir.path().display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "First pet?".into(),
            recovery_answer: "Mochi".into(),
        })
        .unwrap();
        start_review(&source_dir.path().display().to_string()).unwrap();
        assert_eq!(
            skip_review_item(DecideRequest {
                candidate_id: -1,
                tags: vec![],
            })
            .unwrap_err()
            .code,
            "candidate_not_pending"
        );
        assert_eq!(current_review_state().unwrap().state, "resumable");
        assert_eq!(fs::read(source).unwrap(), b"original");
    }

    #[test]
    fn switching_sources_retains_the_first_pending_review() {
        let _session_guard = library::test_session_guard();
        let library_dir = tempdir().unwrap();
        let first_source = tempdir().unwrap();
        let second_source = tempdir().unwrap();
        let first_file = first_source.path().join("first.jpg");
        let second_file = second_source.path().join("second.jpg");
        fs::write(&first_file, b"first original").unwrap();
        fs::write(&second_file, b"second original").unwrap();
        library::setup_library(library::SetupLibraryRequest {
            folder_path: library_dir.path().display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "First pet?".into(),
            recovery_answer: "Mochi".into(),
        })
        .unwrap();
        start_review(&first_source.path().display().to_string()).unwrap();
        start_review(&second_source.path().display().to_string()).unwrap();
        assert_eq!(
            current_review_state().unwrap().source_path.as_deref(),
            Some(
                second_source
                    .path()
                    .canonicalize()
                    .unwrap()
                    .to_str()
                    .unwrap()
            )
        );
        let second_candidate = library::with_catalogue(|connection, _| {
            connection
                .query_row(
                    "SELECT c.id FROM review_candidates c JOIN review_sessions s ON s.id = c.session_id WHERE s.source_path = ?1",
                    [second_source.path().canonicalize().unwrap().display().to_string()],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)
        })
        .unwrap();
        skip_review_item(DecideRequest {
            candidate_id: second_candidate,
            tags: vec![],
        })
        .unwrap();
        assert_eq!(
            current_review_state().unwrap().source_path.as_deref(),
            Some(
                second_source
                    .path()
                    .canonicalize()
                    .unwrap()
                    .to_str()
                    .unwrap()
            )
        );
        let first_is_still_pending = library::with_catalogue(|connection, _| {
            connection
                .query_row(
                    "SELECT count(*) FROM review_candidates c JOIN review_sessions s ON s.id = c.session_id WHERE s.source_path = ?1 AND c.decision IS NULL",
                    [first_source.path().canonicalize().unwrap().display().to_string()],
                    |row| row.get::<_, u64>(0),
                )
                .map_err(database_error)
        })
        .unwrap();
        assert_eq!(first_is_still_pending, 1);
        assert_eq!(fs::read(first_file).unwrap(), b"first original");
        assert_eq!(fs::read(second_file).unwrap(), b"second original");
    }
}

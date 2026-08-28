use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use rusqlite::params;
use serde::Serialize;

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
    library::with_catalogue(|connection, _| {
        let row = connection.query_row(
            "SELECT source_path, (SELECT count(*) FROM review_candidates WHERE session_id = review_sessions.id AND decision IS NULL) FROM review_sessions WHERE state = 'active' ORDER BY id DESC LIMIT 1",
            [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)),
        );
        match row {
            Ok((source_path, candidate_count)) => Ok(ReviewState {
                state: "resumable",
                source_path: Some(source_path),
                candidate_count,
                message: "Resume the safe read-only review. Originals are never modified.".into(),
            }),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(ReviewState {
                state: "none",
                source_path: None,
                candidate_count: 0,
                message: "No unfinished review is available.".into(),
            }),
            Err(error) => Err(database_error(error)),
        }
    })
}

pub fn start_review(source_path: &str) -> Result<ReviewState, ReviewError> {
    library::with_catalogue(|connection, library_path| {
        let source = canonical_directory(source_path)?;
        let library = library_path.canonicalize().map_err(|_| ReviewError {
            code: "library_unavailable",
            message: "The protected library is unavailable.".into(),
        })?;
        if source == library || source.starts_with(&library) || library.starts_with(&source) {
            return Err(ReviewError { code: "source_overlaps_library", message: "The import source must be separate from the protected library. Nothing was changed.".into() });
        }
        let source_text = source.display().to_string();
        if let Ok((id, count)) = connection.query_row("SELECT id, (SELECT count(*) FROM review_candidates WHERE session_id = review_sessions.id AND decision IS NULL) FROM review_sessions WHERE source_path = ?1 AND state = 'active'", [&source_text], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, u64>(1)?))) {
            return Ok(ReviewState { state: "resumable", source_path: Some(source_text), candidate_count: count, message: format!("Resuming the existing review session ({id}). Originals are never modified.") });
        }
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

fn canonical_directory(path: &str) -> Result<PathBuf, ReviewError> {
    let path = PathBuf::from(path.trim());
    if path.as_os_str().is_empty() {
        return Err(ReviewError {
            code: "missing_source",
            message: "Choose an import folder first.".into(),
        });
    }
    let metadata = fs::metadata(&path).map_err(|_| ReviewError {
        code: "source_unavailable",
        message: "The selected import folder is unavailable.".into(),
    })?;
    if !metadata.is_dir() {
        return Err(ReviewError {
            code: "not_a_folder",
            message: "Select a folder, not a file.".into(),
        });
    }
    path.canonicalize().map_err(|_| ReviewError {
        code: "source_unavailable",
        message: "The selected import folder is unavailable.".into(),
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
        .map_err(|_| ReviewError {
            code: "source_unavailable",
            message: "The import folder could not be read.".into(),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ReviewError {
            code: "source_unavailable",
            message: "The import folder could not be read.".into(),
        })?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|_| ReviewError {
            code: "source_unavailable",
            message: "The import folder could not be read.".into(),
        })?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if entry.file_name() != ".photo-handler" {
                visit(root, &path, candidates)?;
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(media_type) = media_type(&path) else {
            continue;
        };
        let metadata = entry.metadata().map_err(|_| ReviewError {
            code: "source_unavailable",
            message: "A source file could not be read.".into(),
        })?;
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
    Ok(())
}

fn media_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "heic" => Some("image"),
        "mp4" | "mov" | "m4v" | "webm" => Some("video"),
        _ => None,
    }
}

fn database_error(error: rusqlite::Error) -> ReviewError {
    ReviewError {
        code: "catalogue_unavailable",
        message: format!("Could not update the protected review catalogue: {error}"),
    }
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
    fn review_commands_require_an_unlocked_library_session() {
        library::lock_library();
        assert_eq!(current_review_state().unwrap_err().code, "library_locked");
    }
}

use std::{fs, path::Path};

use chrono::NaiveDate;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::{params_from_iter, types::Value, Connection};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::library::{self, SetupLibraryError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryRequest {
    #[serde(default)]
    pub date_field: DateField,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub media_type: Option<MediaType>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum DateField {
    #[default]
    Selected,
    Original,
}

impl DateField {
    fn column(self) -> &'static str {
        match self {
            Self::Selected => "d.effective_import_date",
            Self::Original => "d.original_media_date",
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Video,
}

impl MediaType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryResult {
    pub items: Vec<SearchLibraryItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryItem {
    pub candidate_id: i64,
    pub filename: String,
    pub media_type: String,
    pub effective_import_date: Option<String>,
    pub original_media_date: Option<String>,
    pub tags: Vec<String>,
    pub preview_url: Option<String>,
    pub preview_state: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSuggestionRequest {
    pub prefix: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSuggestionResult {
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchError {
    pub code: &'static str,
    pub message: String,
}

impl From<SetupLibraryError> for SearchError {
    fn from(value: SetupLibraryError) -> Self {
        Self {
            code: value.code,
            message: value.message,
        }
    }
}

struct CatalogueItem {
    candidate_id: i64,
    destination_path: String,
    media_type: String,
    effective_import_date: Option<String>,
    original_media_date: Option<String>,
    tags: Vec<String>,
}

pub fn search_library(
    app: tauri::AppHandle,
    request: SearchLibraryRequest,
) -> Result<SearchLibraryResult, SearchError> {
    let start_date = validate_date(request.start_date.as_deref())?;
    let end_date = validate_date(request.end_date.as_deref())?;
    if start_date.as_deref() > end_date.as_deref() {
        return Err(error(
            "invalid_date_range",
            "The start date must be on or before the end date.",
        ));
    }
    let media_type = request.media_type.map(MediaType::as_str);
    let tags = normalize_tags(&request.tags);
    let items = library::with_catalogue(|connection, root| {
        query_imported_items(
            connection,
            start_date.as_deref(),
            end_date.as_deref(),
            media_type,
            request.date_field,
            &tags,
        )
        .map(|items| (items, root.to_path_buf()))
    })?;
    Ok(SearchLibraryResult {
        items: items
            .0
            .into_iter()
            .map(|item| {
                let (preview_url, preview_state) =
                    safe_preview_url(&app, &items.1, Path::new(&item.destination_path));
                SearchLibraryItem {
                    candidate_id: item.candidate_id,
                    filename: Path::new(&item.destination_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    media_type: item.media_type,
                    effective_import_date: item.effective_import_date,
                    original_media_date: item.original_media_date,
                    tags: item.tags,
                    preview_url,
                    preview_state,
                }
            })
            .collect(),
    })
}

fn query_imported_items(
    connection: &Connection,
    start: Option<&str>,
    end: Option<&str>,
    media_type: Option<&str>,
    date_field: DateField,
    tags: &[String],
) -> Result<Vec<CatalogueItem>, SearchError> {
    let date_column = date_field.column();
    let mut sql = format!("SELECT d.candidate_id, d.destination_path, c.media_type, d.effective_import_date, d.original_media_date FROM item_decisions d JOIN review_candidates c ON c.id = d.candidate_id WHERE d.decision = 'imported' AND d.destination_path IS NOT NULL AND (?1 IS NULL OR {date_column} >= ?1) AND (?2 IS NULL OR {date_column} <= ?2) AND (?3 IS NULL OR c.media_type = ?3)");
    for index in 0..tags.len() {
        sql.push_str(&format!(" AND EXISTS (SELECT 1 FROM candidate_tags ct JOIN tags t ON t.id = ct.tag_id WHERE ct.candidate_id = d.candidate_id AND t.normalized_name = ?{})", index + 4));
    }
    sql.push_str(&format!(" ORDER BY {date_column} DESC, d.candidate_id ASC"));
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let mut values = vec![
        start.map(str::to_owned).into(),
        end.map(str::to_owned).into(),
        media_type.map(str::to_owned).into(),
    ];
    values.extend(tags.iter().cloned().map(Value::from));
    let rows = statement
        .query_map(params_from_iter(values), |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(database_error)?;
    rows.map(|row| {
        let (
            candidate_id,
            destination_path,
            media_type,
            effective_import_date,
            original_media_date,
        ) = row.map_err(database_error)?;
        Ok(CatalogueItem {
            candidate_id,
            destination_path,
            media_type,
            effective_import_date,
            original_media_date,
            tags: tags_for_item(connection, candidate_id)?,
        })
    })
    .collect()
}

pub fn suggest_library_tags(
    request: TagSuggestionRequest,
) -> Result<TagSuggestionResult, SearchError> {
    let prefix = normalize_tag(&request.prefix);
    if prefix.chars().count() < 2 {
        return Ok(TagSuggestionResult { tags: vec![] });
    }
    let tags = library::with_catalogue(|connection, _| {
        let mut statement = connection.prepare("SELECT DISTINCT t.normalized_name FROM tags t JOIN candidate_tags ct ON ct.tag_id = t.id JOIN item_decisions d ON d.candidate_id = ct.candidate_id WHERE d.decision = 'imported' AND d.destination_path IS NOT NULL AND t.normalized_name LIKE ?1 ESCAPE '\\' ORDER BY t.normalized_name LIMIT 12").map_err(database_error)?;
        let results = statement
            .query_map([format!("{}%", escape_like(&prefix))], |row| row.get(0))
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        Ok::<_, SearchError>(results)
    })?;
    Ok(TagSuggestionResult { tags })
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|tag| normalize_tag(tag))
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn normalize_tag(tag: &str) -> String {
    tag.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn tags_for_item(connection: &Connection, candidate_id: i64) -> Result<Vec<String>, SearchError> {
    let mut statement = connection.prepare("SELECT t.normalized_name FROM tags t JOIN candidate_tags ct ON ct.tag_id = t.id WHERE ct.candidate_id = ?1 ORDER BY t.normalized_name").map_err(database_error)?;
    let tags = statement
        .query_map([candidate_id], |row| row.get(0))
        .map_err(database_error)?
        .collect::<Result<_, _>>()
        .map_err(database_error)?;
    Ok(tags)
}

pub(crate) fn safe_preview_url(
    app: &tauri::AppHandle,
    root: &Path,
    destination: &Path,
) -> (Option<String>, &'static str) {
    let root = match root.canonicalize() {
        Ok(root) => root,
        Err(_) => return (None, "unavailable"),
    };
    let _metadata = match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            metadata
        }
        _ => return (None, "unavailable"),
    };
    if fs::File::open(destination).is_err() {
        return (None, "unavailable");
    }
    let destination = match destination.canonicalize() {
        Ok(destination)
            if destination.starts_with(&root)
                && !destination.starts_with(root.join(".photo-handler")) =>
        {
            destination
        }
        _ => return (None, "unavailable"),
    };
    if app.asset_protocol_scope().allow_file(&destination).is_err() {
        return (None, "unavailable");
    }
    let Some(path) = destination.to_str() else {
        return (None, "unavailable");
    };
    (
        Some(format!(
            "asset://localhost/{}",
            utf8_percent_encode(path, NON_ALPHANUMERIC)
        )),
        "available",
    )
}

fn validate_date(value: Option<&str>) -> Result<Option<String>, SearchError> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
                .map(|date| date.format("%Y-%m-%d").to_string())
                .map_err(|_| error("invalid_date", "Use dates in YYYY-MM-DD format."))
        })
        .transpose()
}
fn error(code: &'static str, message: impl Into<String>) -> SearchError {
    SearchError {
        code,
        message: message.into(),
    }
}
fn database_error(value: rusqlite::Error) -> SearchError {
    error(
        "catalogue_unavailable",
        format!("Could not search the protected library catalogue: {value}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn date_validation_rejects_bad_ranges() {
        assert!(validate_date(Some("2026-02-30")).is_err());
    }

    #[test]
    fn query_returns_imported_items_only_with_date_and_type_filters() {
        let _session_guard = library::test_session_guard();
        let directory = tempdir().unwrap();
        library::setup_library(library::SetupLibraryRequest {
            folder_path: directory.path().display().to_string(),
            password: "correct horse battery staple".into(),
            password_confirmation: "correct horse battery staple".into(),
            recovery_question: "pet".into(),
            recovery_answer: "Mochi".into(),
        })
        .unwrap();
        library::with_catalogue(|connection, _| {
            connection.execute_batch("INSERT INTO review_sessions (id, source_path, state) VALUES (1, 'source', 'complete'); INSERT INTO review_candidates (id, session_id, relative_path, file_size, modified_at, media_type, decision) VALUES (1, 1, 'one.jpg', 1, 0, 'image', 'imported'), (2, 1, 'two.mp4', 1, 0, 'video', 'imported'), (3, 1, 'skip.jpg', 1, 0, 'image', 'skipped'); INSERT INTO item_decisions (candidate_id, decision, destination_path, effective_import_date, original_media_date) VALUES (1, 'imported', 'one.jpg', '2026-08-20', '2020-07-14'), (2, 'imported', 'two.mp4', '2026-08-21', NULL), (3, 'skipped', NULL, '2026-08-22', '2019-01-01'); INSERT INTO tags (id, normalized_name) VALUES (1, 'summer'), (2, 'family'), (3, 'skipped-only'); INSERT INTO candidate_tags (candidate_id, tag_id) VALUES (1, 1), (1, 2), (3, 3);").map_err(database_error)?;
            Ok::<(), SearchError>(())
        }).unwrap();
        let items = library::with_catalogue(|connection, _| {
            query_imported_items(
                connection,
                Some("2026-08-20"),
                Some("2026-08-20"),
                Some("image"),
                DateField::Selected,
                &[],
            )
        })
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].candidate_id, 1);
        assert_eq!(items[0].tags, ["family", "summer"]);
        let original_items = library::with_catalogue(|connection, _| {
            query_imported_items(
                connection,
                Some("2020-07-14"),
                Some("2020-07-14"),
                None,
                DateField::Original,
                &["summer".into(), "family".into()],
            )
        })
        .unwrap();
        assert_eq!(original_items.len(), 1);
        assert_eq!(original_items[0].candidate_id, 1);
        assert_eq!(
            suggest_library_tags(TagSuggestionRequest {
                prefix: "su".into()
            })
            .unwrap()
            .tags,
            ["summer"]
        );
        assert!(
            suggest_library_tags(TagSuggestionRequest { prefix: "s".into() })
                .unwrap()
                .tags
                .is_empty()
        );
        assert!(suggest_library_tags(TagSuggestionRequest {
            prefix: "sk".into()
        })
        .unwrap()
        .tags
        .is_empty());
        library::lock_library();
    }
}

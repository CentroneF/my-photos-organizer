use std::{
    fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDate;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::{params_from_iter, types::Value, Connection};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::library::{self, SetupLibraryError};
use crate::review::ReviewMetadata;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLibraryRequest {
    pub imported_start_date: Option<String>,
    pub imported_end_date: Option<String>,
    pub captured_start_date: Option<String>,
    pub captured_end_date: Option<String>,
    #[serde(default = "default_media_types")]
    pub media_types: Vec<MediaType>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
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

fn default_media_types() -> Vec<MediaType> {
    vec![MediaType::Image, MediaType::Video]
}

fn normalize_media_types(media_types: Vec<MediaType>) -> Vec<MediaType> {
    let mut normalized = Vec::new();
    for media_type in media_types {
        if !normalized.contains(&media_type) {
            normalized.push(media_type);
        }
    }
    normalized
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
pub struct MediaDetailsRequest {
    pub candidate_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMediaTagsRequest {
    pub candidate_id: i64,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedMediaActionRequest {
    pub candidate_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotateManagedMediaRequest {
    pub candidate_id: i64,
    pub direction: RotationDirection,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum RotationDirection {
    Left,
    Right,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteManagedMediaRequest {
    pub candidate_id: i64,
    pub confirmed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteManagedMediaResult {
    pub candidate_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMediaTagsResult {
    pub candidate_id: i64,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDetails {
    pub state: &'static str,
    pub candidate_id: i64,
    pub filename: String,
    pub media_type: String,
    pub tags: Vec<String>,
    pub preview_url: Option<String>,
    pub preview_state: &'static str,
    pub rotation_supported: bool,
    pub metadata: ReviewMetadata,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListLibraryTagsRequest {
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListLibraryTagsResult {
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentTagsResult {
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

struct ResolvedImportedItem {
    candidate_id: i64,
    destination: std::path::PathBuf,
    media_type: String,
    tags: Vec<String>,
}

/// Resolves only current managed copies. Callers never receive or accept arbitrary paths.
fn resolve_active_imported_item(
    connection: &Connection,
    root: &Path,
    candidate_id: i64,
) -> Result<ResolvedImportedItem, SearchError> {
    let (destination_path, media_type): (String, String) = connection
        .query_row(
            "SELECT d.destination_path, c.media_type FROM item_decisions d JOIN review_candidates c ON c.id = d.candidate_id WHERE d.candidate_id = ?1 AND d.decision = 'imported' AND d.destination_path IS NOT NULL AND d.replaced_by_candidate_id IS NULL",
            [candidate_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|value| match value {
            rusqlite::Error::QueryReturnedNoRows => error(
                "media_unavailable",
                "This managed media item is no longer available in the active library.",
            ),
            other => database_error(other),
        })?;
    let destination = validate_managed_file(root, Path::new(&destination_path))?;
    Ok(ResolvedImportedItem {
        candidate_id,
        destination,
        media_type,
        tags: tags_for_item(connection, candidate_id)?,
    })
}

fn validate_managed_file(
    root: &Path,
    destination: &Path,
) -> Result<std::path::PathBuf, SearchError> {
    let root = root.canonicalize().map_err(|_| {
        error(
            "media_unavailable",
            "The protected library is unavailable. Unlock it again and retry.",
        )
    })?;
    let metadata = fs::symlink_metadata(destination).map_err(|_| {
        error(
            "media_unavailable",
            "The managed media file is missing or cannot be read.",
        )
    })?;
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || fs::File::open(destination).is_err()
    {
        return Err(error(
            "media_unavailable",
            "The managed media file is missing or cannot be read.",
        ));
    }
    let destination = destination.canonicalize().map_err(|_| {
        error(
            "media_unavailable",
            "The managed media file is missing or cannot be read.",
        )
    })?;
    if !destination.starts_with(&root) || destination.starts_with(root.join(".photo-handler")) {
        return Err(error(
            "media_unavailable",
            "The selected file is outside the managed media library.",
        ));
    }
    Ok(destination)
}

pub fn media_details(
    app: tauri::AppHandle,
    request: MediaDetailsRequest,
) -> Result<MediaDetails, SearchError> {
    let (item, root) = library::with_catalogue(|connection, root| {
        resolve_active_imported_item(connection, root, request.candidate_id)
            .map(|item| (item, root.to_path_buf()))
    })?;
    let (preview_url, preview_state) = safe_preview_url(&app, &root, &item.destination);
    let preview_url = preview_url.map(|url| versioned_preview_url(url, &item.destination));
    let available = preview_state == "available";
    Ok(MediaDetails {
        state: if available {
            "available"
        } else {
            "unavailable"
        },
        candidate_id: item.candidate_id,
        filename: item
            .destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        media_type: item.media_type,
        tags: item.tags,
        preview_url,
        preview_state,
        rotation_supported: supports_rotation(&item.destination),
        metadata: crate::review::review_metadata(&item.destination),
        message: if available {
            "Managed media is ready for preview.".into()
        } else {
            "This managed media file is unavailable or cannot be previewed safely.".into()
        },
    })
}

fn versioned_preview_url(url: String, destination: &Path) -> String {
    let revision = fs::metadata(destination)
        .and_then(|metadata| metadata.modified())
        .and_then(|modified| {
            modified
                .duration_since(UNIX_EPOCH)
                .map_err(std::io::Error::other)
        })
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{url}?revision={revision}")
}

fn supports_rotation(destination: &Path) -> bool {
    image::ImageReader::open(destination)
        .ok()
        .and_then(|reader| reader.with_guessed_format().ok())
        .and_then(|reader| reader.format())
        .is_some_and(|format| {
            matches!(
                format,
                image::ImageFormat::Jpeg | image::ImageFormat::Png | image::ImageFormat::WebP
            )
        })
}

/// Replaces an imported item's tag set after proving that its managed copy is still safe.
pub fn update_media_tags(
    request: UpdateMediaTagsRequest,
) -> Result<UpdateMediaTagsResult, SearchError> {
    let tags = normalize_tags(&request.tags);
    library::with_catalogue(|connection, root| {
        resolve_active_imported_item(connection, root, request.candidate_id)?;
        let transaction = connection.unchecked_transaction().map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM candidate_tags WHERE candidate_id = ?1",
                [request.candidate_id],
            )
            .map_err(database_error)?;
        for tag in &tags {
            transaction
                .execute(
                    "INSERT INTO tags (normalized_name) VALUES (?1) ON CONFLICT(normalized_name) DO NOTHING",
                    [tag],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT INTO candidate_tags (candidate_id, tag_id) SELECT ?1, id FROM tags WHERE normalized_name = ?2",
                    rusqlite::params![request.candidate_id, tag],
                )
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(UpdateMediaTagsResult {
            candidate_id: request.candidate_id,
            tags,
        })
    })
}

/// Returns only a validated managed path for native-only copy and reveal commands.
pub fn managed_media_path(
    request: ManagedMediaActionRequest,
) -> Result<std::path::PathBuf, SearchError> {
    library::with_catalogue(|connection, root| {
        resolve_active_imported_item(connection, root, request.candidate_id)
            .map(|item| item.destination)
    })
}

/// Rotates a validated managed image, overwriting only the managed copy.
pub fn rotate_managed_media(
    app: tauri::AppHandle,
    request: RotateManagedMediaRequest,
) -> Result<MediaDetails, SearchError> {
    let item = library::with_catalogue(|connection, root| {
        resolve_active_imported_item(connection, root, request.candidate_id)
    })?;
    if item.media_type != "image" {
        return Err(error(
            "rotation_unsupported",
            "Only supported managed image files can be rotated.",
        ));
    }
    rotate_image_file(&item.destination, request.direction)?;
    media_details(
        app,
        MediaDetailsRequest {
            candidate_id: item.candidate_id,
        },
    )
}

/// Moves a validated managed copy to Trash before hiding it from active search results.
pub fn delete_managed_media(
    request: DeleteManagedMediaRequest,
) -> Result<DeleteManagedMediaResult, SearchError> {
    if !request.confirmed {
        return Err(error(
            "confirmation_required",
            "Confirm moving the managed copy to Trash before deleting it.",
        ));
    }
    delete_managed_media_with_trash(request, |path| {
        trash::delete(path).map_err(|value| value.to_string())
    })
}

fn delete_managed_media_with_trash(
    request: DeleteManagedMediaRequest,
    move_to_trash: impl Fn(&Path) -> Result<(), String>,
) -> Result<DeleteManagedMediaResult, SearchError> {
    if !request.confirmed {
        return Err(error(
            "confirmation_required",
            "Confirm moving the managed copy to Trash before deleting it.",
        ));
    }
    library::with_catalogue(|connection, root| {
        let item = resolve_active_imported_item(connection, root, request.candidate_id)?;
        move_to_trash(&item.destination).map_err(|_| {
            error(
                "trash_failed",
                "Could not move the managed copy to Trash. It remains in the library.",
            )
        })?;
        connection.execute(
            "UPDATE item_decisions SET destination_path = NULL WHERE candidate_id = ?1 AND decision = 'imported' AND destination_path IS NOT NULL AND replaced_by_candidate_id IS NULL",
            [item.candidate_id],
        ).map_err(database_error)?;
        Ok(DeleteManagedMediaResult {
            candidate_id: item.candidate_id,
        })
    })
}

fn rotate_image_file(destination: &Path, direction: RotationDirection) -> Result<(), SearchError> {
    let reader = image::ImageReader::open(destination)
        .map_err(|_| {
            error(
                "rotation_unsupported",
                "This managed image cannot be decoded for rotation.",
            )
        })?
        .with_guessed_format()
        .map_err(|_| {
            error(
                "rotation_unsupported",
                "This managed image format is unsupported for rotation.",
            )
        })?;
    let format = reader.format().ok_or_else(|| {
        error(
            "rotation_unsupported",
            "This managed image format is unsupported for rotation.",
        )
    })?;
    if !matches!(
        format,
        image::ImageFormat::Jpeg | image::ImageFormat::Png | image::ImageFormat::WebP
    ) {
        return Err(error(
            "rotation_unsupported",
            "This managed image format is unsupported for rotation.",
        ));
    }
    let image = reader.decode().map_err(|_| {
        error(
            "rotation_unsupported",
            "This managed image cannot be decoded for rotation.",
        )
    })?;
    let rotated = match direction {
        RotationDirection::Left => image.rotate270(),
        RotationDirection::Right => image.rotate90(),
    };
    let temporary = rotation_temporary_path(destination);
    let output = fs::File::create(&temporary).map_err(|_| {
        error(
            "rotation_failed",
            "Could not prepare a safe replacement for the managed copy.",
        )
    })?;
    let mut writer = BufWriter::new(output);
    if rotated.write_to(&mut writer, format).is_err() || writer.flush().is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(error(
            "rotation_failed",
            "Could not write the rotated managed copy; the original was preserved.",
        ));
    }
    let output = writer.into_inner().map_err(|_| {
        let _ = fs::remove_file(&temporary);
        error(
            "rotation_failed",
            "Could not finish the rotated managed copy; the original was preserved.",
        )
    })?;
    if output.sync_all().is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(error(
            "rotation_failed",
            "Could not safely finish the rotated managed copy; the original was preserved.",
        ));
    }
    replace_with_rotated_file(&temporary, destination)
}

fn rotation_temporary_path(destination: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let extension = destination
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("tmp");
    destination.with_file_name(format!(
        ".photo-handler-rotate-{}-{}.{}",
        std::process::id(),
        stamp,
        extension
    ))
}

fn replace_with_rotated_file(temporary: &Path, destination: &Path) -> Result<(), SearchError> {
    #[cfg(not(target_os = "windows"))]
    {
        fs::rename(temporary, destination).map_err(|_| {
            let _ = fs::remove_file(temporary);
            error(
                "rotation_failed",
                "Could not replace the managed copy; the original was preserved.",
            )
        })
    }
    #[cfg(target_os = "windows")]
    {
        let backup = destination.with_extension("photo-handler-rotate-backup");
        fs::rename(destination, &backup).map_err(|_| {
            error(
                "rotation_failed",
                "Could not prepare the managed copy for replacement.",
            )
        })?;
        if fs::rename(temporary, destination).is_err() {
            let _ = fs::rename(&backup, destination);
            let _ = fs::remove_file(temporary);
            return Err(error(
                "rotation_failed",
                "Could not replace the managed copy; the original was preserved.",
            ));
        }
        fs::remove_file(backup).map_err(|_| {
            error(
                "rotation_failed",
                "The rotated managed copy was saved but its backup needs attention.",
            )
        })
    }
}

pub fn search_library(
    app: tauri::AppHandle,
    request: SearchLibraryRequest,
) -> Result<SearchLibraryResult, SearchError> {
    let imported_start_date = validate_date(request.imported_start_date.as_deref())?;
    let imported_end_date = validate_date(request.imported_end_date.as_deref())?;
    validate_date_range(imported_start_date.as_deref(), imported_end_date.as_deref())?;
    let captured_start_date = validate_date(request.captured_start_date.as_deref())?;
    let captured_end_date = validate_date(request.captured_end_date.as_deref())?;
    validate_date_range(captured_start_date.as_deref(), captured_end_date.as_deref())?;
    let media_types = normalize_media_types(request.media_types);
    let tags = normalize_tags(&request.tags);
    let items = library::with_catalogue(|connection, root| {
        query_imported_items(
            connection,
            imported_start_date.as_deref(),
            imported_end_date.as_deref(),
            captured_start_date.as_deref(),
            captured_end_date.as_deref(),
            &media_types,
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
    imported_start: Option<&str>,
    imported_end: Option<&str>,
    captured_start: Option<&str>,
    captured_end: Option<&str>,
    media_types: &[MediaType],
    tags: &[String],
) -> Result<Vec<CatalogueItem>, SearchError> {
    if media_types.is_empty() {
        return Ok(vec![]);
    }
    let mut sql = "SELECT d.candidate_id, d.destination_path, c.media_type, d.effective_import_date, d.original_media_date FROM item_decisions d JOIN review_candidates c ON c.id = d.candidate_id WHERE d.decision = 'imported' AND d.destination_path IS NOT NULL AND d.replaced_by_candidate_id IS NULL AND (?1 IS NULL OR d.effective_import_date >= ?1) AND (?2 IS NULL OR d.effective_import_date <= ?2) AND (?3 IS NULL OR d.original_media_date >= ?3) AND (?4 IS NULL OR d.original_media_date <= ?4) AND c.media_type IN (".to_owned();
    for index in 0..media_types.len() {
        if index > 0 {
            sql.push_str(", ");
        }
        sql.push_str(&format!("?{}", index + 5));
    }
    sql.push(')');
    for index in 0..tags.len() {
        sql.push_str(&format!(" AND EXISTS (SELECT 1 FROM candidate_tags ct JOIN tags t ON t.id = ct.tag_id WHERE ct.candidate_id = d.candidate_id AND t.normalized_name = ?{})", index + 5 + media_types.len()));
    }
    sql.push_str(" ORDER BY d.effective_import_date DESC, d.candidate_id ASC");
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let mut values = vec![
        imported_start.map(str::to_owned).into(),
        imported_end.map(str::to_owned).into(),
        captured_start.map(str::to_owned).into(),
        captured_end.map(str::to_owned).into(),
    ];
    values.extend(
        media_types
            .iter()
            .map(|media_type| Value::from(media_type.as_str().to_owned())),
    );
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

pub fn list_library_tags(
    request: ListLibraryTagsRequest,
) -> Result<ListLibraryTagsResult, SearchError> {
    let query = request
        .query
        .as_deref()
        .map(normalize_tag)
        .unwrap_or_default();
    let tags = library::with_catalogue(|connection, _| {
        let mut statement = connection.prepare("SELECT t.normalized_name, COUNT(*) AS frequency FROM tags t JOIN candidate_tags ct ON ct.tag_id = t.id JOIN item_decisions d ON d.candidate_id = ct.candidate_id WHERE d.decision = 'imported' AND d.destination_path IS NOT NULL AND d.replaced_by_candidate_id IS NULL AND (?1 = '' OR t.normalized_name LIKE ?2 ESCAPE '\\') GROUP BY t.id, t.normalized_name ORDER BY frequency DESC, t.normalized_name ASC LIMIT 10").map_err(database_error)?;
        let results = statement
            .query_map([&query, &format!("%{}%", escape_like(&query))], |row| {
                row.get(0)
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        Ok::<_, SearchError>(results)
    })?;
    Ok(ListLibraryTagsResult { tags })
}

pub fn recent_library_tags() -> Result<RecentTagsResult, SearchError> {
    let tags = library::with_catalogue(|connection, _| {
        let mut statement = connection
            .prepare(
                "SELECT t.normalized_name, MAX(d.decided_at) AS last_used \
                 FROM tags t \
                 JOIN candidate_tags ct ON ct.tag_id = t.id \
                 JOIN item_decisions d ON d.candidate_id = ct.candidate_id \
                 WHERE d.decision = 'imported' AND d.destination_path IS NOT NULL AND d.replaced_by_candidate_id IS NULL \
                 GROUP BY t.id, t.normalized_name \
                 ORDER BY last_used DESC, t.normalized_name ASC LIMIT 5",
            )
            .map_err(database_error)?;
        let tags = statement
            .query_map([], |row| row.get(0))
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        Ok::<_, SearchError>(tags)
    })?;
    Ok(RecentTagsResult { tags })
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags.iter().map(|tag| normalize_tag(tag)) {
        if !tag.is_empty() && !normalized.contains(&tag) {
            normalized.push(tag);
        }
    }
    normalized
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

fn validate_date_range(start: Option<&str>, end: Option<&str>) -> Result<(), SearchError> {
    if start > end {
        return Err(error(
            "invalid_date_range",
            "The start date must be on or before the end date.",
        ));
    }
    Ok(())
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
                None,
                None,
                &[MediaType::Image],
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
                None,
                None,
                Some("2020-07-14"),
                Some("2020-07-14"),
                &[MediaType::Image, MediaType::Video],
                &["summer".into(), "family".into()],
            )
        })
        .unwrap();
        assert_eq!(original_items.len(), 1);
        assert_eq!(original_items[0].candidate_id, 1);
        let combined_items = library::with_catalogue(|connection, _| {
            query_imported_items(
                connection,
                Some("2026-08-20"),
                Some("2026-08-20"),
                Some("2020-07-14"),
                Some("2020-07-14"),
                &[MediaType::Image, MediaType::Video],
                &[],
            )
        })
        .unwrap();
        assert_eq!(combined_items.len(), 1);
        assert_eq!(combined_items[0].candidate_id, 1);
        let null_captured_items = library::with_catalogue(|connection, _| {
            query_imported_items(
                connection,
                Some("2026-08-21"),
                Some("2026-08-21"),
                Some("2020-01-01"),
                Some("2026-12-31"),
                &[MediaType::Image, MediaType::Video],
                &[],
            )
        })
        .unwrap();
        assert!(null_captured_items.is_empty());
        assert_eq!(
            list_library_tags(ListLibraryTagsRequest {
                query: Some("mm".into())
            })
            .unwrap()
            .tags,
            ["summer"]
        );
        assert!(list_library_tags(ListLibraryTagsRequest {
            query: Some("sk".into())
        })
        .unwrap()
        .tags
        .is_empty());
        assert_eq!(
            recent_library_tags().unwrap().tags,
            ["family", "summer"],
            "recent review tags are imported-only and deterministically ordered"
        );
        library::lock_library();
    }

    #[test]
    fn library_tag_list_ranks_literal_substrings_from_active_imports_only() {
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
            connection.execute_batch("INSERT INTO review_sessions (id, source_path, state) VALUES (1, 'source', 'complete');
                INSERT INTO review_candidates (id, session_id, relative_path, file_size, modified_at, media_type, decision) VALUES
                (1,1,'1.jpg',1,0,'image','imported'),(2,1,'2.jpg',1,0,'image','imported'),(3,1,'3.jpg',1,0,'image','imported'),(4,1,'4.jpg',1,0,'image','imported'),(5,1,'5.jpg',1,0,'image','imported'),(6,1,'6.jpg',1,0,'image','imported'),(7,1,'7.jpg',1,0,'image','imported'),(8,1,'8.jpg',1,0,'image','imported'),(9,1,'9.jpg',1,0,'image','imported'),(10,1,'10.jpg',1,0,'image','imported'),(11,1,'11.jpg',1,0,'image','imported'),(12,1,'12.jpg',1,0,'image','imported'),(13,1,'13.jpg',1,0,'image','imported'),(14,1,'14.jpg',1,0,'image','skipped'),(15,1,'15.jpg',1,0,'image','imported');
                INSERT INTO item_decisions (candidate_id, decision, destination_path, effective_import_date, replaced_by_candidate_id) VALUES
                (1,'imported','1.jpg','2026-08-20',NULL),(2,'imported','2.jpg','2026-08-20',NULL),(3,'imported','3.jpg','2026-08-20',NULL),(4,'imported','4.jpg','2026-08-20',NULL),(5,'imported','5.jpg','2026-08-20',NULL),(6,'imported','6.jpg','2026-08-20',NULL),(7,'imported','7.jpg','2026-08-20',NULL),(8,'imported','8.jpg','2026-08-20',NULL),(9,'imported','9.jpg','2026-08-20',NULL),(10,'imported','10.jpg','2026-08-20',NULL),(11,'imported','11.jpg','2026-08-20',NULL),(12,'imported','12.jpg','2026-08-20',NULL),(13,'imported','13.jpg','2026-08-20',NULL),(14,'skipped',NULL,'2026-08-20',NULL),(15,'imported','15.jpg','2026-08-20',1);
                INSERT INTO tags (id, normalized_name) VALUES (1,'alpha'),(2,'beta'),(3,'middle tag'),(4,'percent%tag'),(5,'under_tag'),(6,'tag-a'),(7,'tag-b'),(8,'tag-c'),(9,'tag-d'),(10,'tag-e'),(11,'tag-f'),(12,'skipped-only'),(13,'replaced-only');
                INSERT INTO candidate_tags (candidate_id, tag_id) VALUES (1,1),(2,1),(3,1),(4,2),(5,2),(6,2),(7,3),(8,4),(9,5),(10,6),(11,7),(12,8),(13,9),(1,10),(2,11),(14,12),(15,13);").map_err(database_error)?;
            Ok::<(), SearchError>(())
        }).unwrap();

        assert_eq!(
            list_library_tags(ListLibraryTagsRequest { query: None })
                .unwrap()
                .tags,
            [
                "alpha",
                "beta",
                "middle tag",
                "percent%tag",
                "tag-a",
                "tag-b",
                "tag-c",
                "tag-d",
                "tag-e",
                "tag-f"
            ],
            "blank queries return the ten most-used tags, with alphabetical ties"
        );
        assert_eq!(
            list_library_tags(ListLibraryTagsRequest {
                query: Some("  DDLE  ".into())
            })
            .unwrap()
            .tags,
            ["middle tag"],
            "queries are whitespace-normalized, case-normalized, and substring-based"
        );
        assert_eq!(
            list_library_tags(ListLibraryTagsRequest {
                query: Some("%".into())
            })
            .unwrap()
            .tags,
            ["percent%tag"],
            "percent is matched literally"
        );
        assert_eq!(
            list_library_tags(ListLibraryTagsRequest {
                query: Some("_".into())
            })
            .unwrap()
            .tags,
            ["under_tag"],
            "underscore is matched literally"
        );
        assert!(list_library_tags(ListLibraryTagsRequest {
            query: Some("only".into())
        })
        .unwrap()
        .tags
        .is_empty());
        library::lock_library();
    }

    #[test]
    fn media_type_selection_supports_each_type_both_duplicates_and_empty() {
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
            connection.execute_batch("INSERT INTO review_sessions (id, source_path, state) VALUES (1, 'source', 'complete'); INSERT INTO review_candidates (id, session_id, relative_path, file_size, modified_at, media_type, decision) VALUES (1, 1, 'one.jpg', 1, 0, 'image', 'imported'), (2, 1, 'two.mp4', 1, 0, 'video', 'imported'); INSERT INTO item_decisions (candidate_id, decision, destination_path, effective_import_date) VALUES (1, 'imported', 'one.jpg', '2026-08-20'), (2, 'imported', 'two.mp4', '2026-08-21');").map_err(database_error)?;
            Ok::<(), SearchError>(())
        }).unwrap();

        for (selection, expected_ids) in [
            (vec![MediaType::Image], vec![1]),
            (vec![MediaType::Video], vec![2]),
            (vec![MediaType::Image, MediaType::Video], vec![2, 1]),
            (vec![MediaType::Image, MediaType::Image], vec![1]),
            (vec![], vec![]),
        ] {
            let selected = normalize_media_types(selection);
            let ids = library::with_catalogue(|connection, _| {
                query_imported_items(connection, None, None, None, None, &selected, &[]).map(
                    |items| {
                        items
                            .into_iter()
                            .map(|item| item.candidate_id)
                            .collect::<Vec<_>>()
                    },
                )
            })
            .unwrap();
            assert_eq!(ids, expected_ids);
        }

        assert_eq!(
            default_media_types(),
            vec![MediaType::Image, MediaType::Video]
        );
        library::lock_library();
    }

    #[test]
    fn date_range_validation_rejects_inverted_ranges() {
        assert!(validate_date_range(Some("2026-08-21"), Some("2026-08-20")).is_err());
        assert!(validate_date_range(Some("2026-08-20"), Some("2026-08-21")).is_ok());
        assert!(validate_date_range(None, Some("2026-08-21")).is_ok());
    }

    #[test]
    fn managed_file_validation_rejects_missing_and_outside_files() {
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let managed = root.path().join("managed.jpg");
        fs::write(&managed, b"managed").unwrap();
        fs::write(outside.path().join("outside.jpg"), b"outside").unwrap();

        assert_eq!(
            validate_managed_file(root.path(), &managed).unwrap(),
            managed.canonicalize().unwrap()
        );
        assert_eq!(
            validate_managed_file(root.path(), &root.path().join("missing.jpg"))
                .unwrap_err()
                .code,
            "media_unavailable"
        );
        assert_eq!(
            validate_managed_file(root.path(), &outside.path().join("outside.jpg"))
                .unwrap_err()
                .code,
            "media_unavailable"
        );
    }

    #[cfg(unix)]
    #[test]
    fn managed_file_validation_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let target = outside.path().join("outside.jpg");
        fs::write(&target, b"outside").unwrap();
        let link = root.path().join("linked.jpg");
        symlink(&target, &link).unwrap();

        assert_eq!(
            validate_managed_file(root.path(), &link).unwrap_err().code,
            "media_unavailable"
        );
    }

    #[test]
    fn tag_updates_normalize_deduplicate_replace_and_validate_managed_items() {
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
        let managed = directory.path().join("managed.jpg");
        fs::write(&managed, b"managed").unwrap();
        library::with_catalogue(|connection, _| {
            connection.execute_batch("INSERT INTO review_sessions (id, source_path, state) VALUES (1, 'source', 'complete'); INSERT INTO review_candidates (id, session_id, relative_path, file_size, modified_at, media_type, decision) VALUES (1, 1, 'managed.jpg', 1, 0, 'image', 'imported'), (2, 1, 'skipped.jpg', 1, 0, 'image', 'skipped'); INSERT INTO item_decisions (candidate_id, decision, destination_path, effective_import_date) VALUES (2, 'skipped', NULL, '2026-09-09'); INSERT INTO tags (id, normalized_name) VALUES (1, 'old'); INSERT INTO candidate_tags (candidate_id, tag_id) VALUES (1, 1);").map_err(database_error)?;
            connection.execute(
                "INSERT INTO item_decisions (candidate_id, decision, destination_path, effective_import_date) VALUES (1, 'imported', ?1, '2026-09-09')",
                [managed.display().to_string()],
            ).map_err(database_error)?;
            Ok::<(), SearchError>(())
        })
        .unwrap();

        let result = update_media_tags(UpdateMediaTagsRequest {
            candidate_id: 1,
            tags: vec![" Summer ".into(), "summer".into(), "Family Album".into()],
        })
        .unwrap();
        assert_eq!(result.tags, ["summer", "family album"]);
        let tags = library::with_catalogue(|connection, _| tags_for_item(connection, 1)).unwrap();
        assert_eq!(tags, ["family album", "summer"]);
        assert_eq!(
            managed_media_path(ManagedMediaActionRequest { candidate_id: 1 }).unwrap(),
            managed.canonicalize().unwrap()
        );
        assert_eq!(
            update_media_tags(UpdateMediaTagsRequest {
                candidate_id: 2,
                tags: vec!["forbidden".into()],
            })
            .unwrap_err()
            .code,
            "media_unavailable"
        );
        library::lock_library();
    }

    #[test]
    fn confirmed_rotation_and_deletion_preserve_managed_media_until_safe() {
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
        let managed = directory.path().join("managed.png");
        let source = tempdir().unwrap();
        let original = source.path().join("original.png");
        let image = image::RgbaImage::from_fn(2, 3, |x, y| image::Rgba([x as u8, y as u8, 0, 255]));
        image.save(&managed).unwrap();
        image.save(&original).unwrap();
        library::with_catalogue(|connection, _| {
            connection.execute_batch("INSERT INTO review_sessions (id, source_path, state) VALUES (1, 'source', 'complete'); INSERT INTO review_candidates (id, session_id, relative_path, file_size, modified_at, media_type, decision) VALUES (1, 1, 'managed.png', 1, 0, 'image', 'imported');").map_err(database_error)?;
            connection.execute("INSERT INTO item_decisions (candidate_id, decision, destination_path, effective_import_date) VALUES (1, 'imported', ?1, '2026-09-09')", [managed.display().to_string()]).map_err(database_error)?;
            Ok::<(), SearchError>(())
        }).unwrap();

        assert_eq!(
            delete_managed_media(DeleteManagedMediaRequest {
                candidate_id: 1,
                confirmed: false
            })
            .unwrap_err()
            .code,
            "confirmation_required"
        );
        assert!(managed.exists());
        rotate_image_file(&managed, RotationDirection::Right).unwrap();
        assert_eq!(image::image_dimensions(&managed).unwrap(), (3, 2));
        assert_eq!(image::image_dimensions(&original).unwrap(), (2, 3));
        assert_eq!(
            delete_managed_media_with_trash(
                DeleteManagedMediaRequest {
                    candidate_id: 1,
                    confirmed: true
                },
                |_| Err("no Trash".into())
            )
            .unwrap_err()
            .code,
            "trash_failed"
        );
        assert!(managed.exists());
        assert!(
            library::with_catalogue(|connection, root| resolve_active_imported_item(
                connection, root, 1
            ))
            .is_ok()
        );
        delete_managed_media_with_trash(
            DeleteManagedMediaRequest {
                candidate_id: 1,
                confirmed: true,
            },
            |path| fs::remove_file(path).map_err(|error| error.to_string()),
        )
        .unwrap();
        assert!(!managed.exists());
        assert!(
            library::with_catalogue(|connection, root| resolve_active_imported_item(
                connection, root, 1
            ))
            .is_err()
        );
        library::lock_library();
    }
}

# Managed library search implementation plan

## Overview

Make Library Search the default experience after a protected library is unlocked. Users will browse their imported media in a safe visual grid and narrow it by a selected-date range, media type, and, in the second slice, suggested normalized tags and the original media date.

## Current State Analysis

The encrypted SQLite catalogue already records review candidates, normalized tags, imported decisions, managed destination paths, media type, and a reviewer-selected `effective_import_date`. The current post-unlock `home` screen instead leads with import-source selection; no search command, query DTO, result grid, or tag-suggestion API exists.

Imported images and videos can reuse the existing dynamic asset protocol, but the current review preview scopes source files only. Search must never grant an asset URL merely because a mutable catalogue row contains a path.

## Desired End State

After unlocking a library, the user immediately sees their imported media in a responsive grid that uses the full application window, with date range and image/video filters. An empty library explains that no media has been imported and presents an action that enters the existing safe import flow. Later, tag suggestions appear after two characters and the user can search by either the preserved original date or selected import date.

### Key Discoveries:

- `library::with_catalogue` is the authenticated encrypted-catalogue gateway and supplies the canonical active-library root (`src-tauri/src/library.rs:302`).
- Imported decisions record `destination_path` and `effective_import_date`; skipped decisions have no destination and must not be search results (`src-tauri/src/review.rs:279`, `src-tauri/src/review.rs:378`).
- Dynamic preview access uses `asset_protocol_scope().allow_file` before returning an `asset://` URL (`src-tauri/src/review.rs:188`).
- The application is a single Dioxus step-state component whose current home screen begins the import workflow (`src/app.rs:880`).

## What We're NOT Doing

- Searching skipped or pending review candidates, source folders, or historical decisions.
- Creating thumbnails/posters, EXIF parsing beyond the existing best-effort date discovery, full-text filename search, maps, duplicate/similarity search, pagination, or a new router.
- Making originals mutable, granting broad filesystem access, or exposing `.photo-handler` files through the asset protocol.
- Backfilling original dates for existing catalogue records by inspecting managed copies; those records retain selected-import-date search only.

## Implementation Approach

Deliver the feature in two frontend-verifiable vertical slices. The first adds a query over imported records, scoped previews, and the default-after-unlock grid with selected-date/type filters and an import-oriented empty state. The second persists the original discovered date for future imports, exposes an explicit date-field selector, and supplies prefix tag suggestions after two characters. Both native query paths use the active encrypted session and return only projected result data, never raw source locations.

## Critical Implementation Details

The preview builder must canonicalize the active library root and each destination, reject unreadable files and symlinks, and require the canonical destination to be beneath that root before calling `allow_file`. A database value alone is not authority to scope a file. Video and unrenderable-image cards retain metadata and a labelled fallback rather than disappearing.

## Phase 1: Default library search and empty state

### Overview

Give an unlocked user a safe, useful managed-media search view immediately, including a clear route into the established import flow when no imports exist.

### Changes Required:

#### 1. Search catalogue module and schema support

**File**: `src-tauri/src/search.rs` (new)

**Intent**: Define the serializable search request/result/error contracts and query imported catalogue records by optional selected-import-date range and media type. Include result tags, safe preview status/URL, and enough metadata to identify each managed item.

**Contract**: `search_library(app, SearchLibraryRequest) -> Result<SearchLibraryResult, SearchError>` runs through `library::with_catalogue`; it returns only `decision = 'imported'` rows with a non-null managed destination. Results are deterministic, tagged rows are deduplicated, and absent filters return all imported items.

**File**: `src-tauri/src/library.rs`

**Intent**: Add the query-supporting indexes required for imported decisions and selected date/type filtering, and migrate the encrypted catalogue transactionally.

**Contract**: Bump `CATALOGUE_FORMAT_VERSION` and create the same schema/indexes for new and upgraded libraries; the migration preserves all existing decisions and follows the established version/identity update contract.

#### 2. Safe native command boundary

**File**: `src-tauri/src/lib.rs`

**Intent**: Register the search command and give it the application handle needed to grant a preview only after native validation.

**Contract**: The public Tauri command name and frontend invocation string remain `search_library`; locked-library errors preserve the current structured `{ code, message }` shape. No capability-file broad filesystem permission is added.

**File**: `src-tauri/src/search.rs` (new)

**Intent**: Limit dynamic asset protocol access to genuine imported media inside the active library.

**Contract**: Canonicalize and verify every destination as a readable regular non-symlink file strictly below the canonical library root before `allow_file`; otherwise return the item with `preview_url: None` and an unavailable-preview state. Never return source paths or scope the `.photo-handler` directory.

#### 3. Default-after-unlock search view

**File**: `src/app.rs`

**Intent**: Replace the import-first unlocked home with a full-window Library Search state loaded after unlock, while keeping the existing import-source/review interactions accessible through an explicit import action.

**Contract**: Add request/response DTOs and signals for start/end selected-import dates, optional `image`/`video` media type, loading/error/empty state, and result cards. The unlocked search state uses the full application viewport rather than the existing centered, narrow `flow-wrap`; invoke `search_library` whenever the user applies filters. The empty state’s Import media action reaches the existing source-picker/review flow without changing original-media safeguards.

**File**: `assets/styles.css`

**Intent**: Add responsive full-window filter and grid/card styles that preserve usable metadata for unavailable previews.

**Contract**: The search workspace fills the application window, with filter controls and the result grid using the available width and height responsively. Image cards use the scoped asset URL; video cards use muted metadata preload with a metadata fallback if the embedded browser cannot render the codec; unavailable image formats (including HEIC) show a labelled placeholder rather than being omitted.

#### 4. Phase-one automated coverage

**File**: `src-tauri/src/search.rs` (new tests)

**Intent**: Prove that the encrypted query and preview boundary are correct before the UI relies on them.

**Contract**: Tests cover imported-only results; date/type intersection; tags attached to results; locked session behavior; empty catalogue; and rejection of destinations that are missing, symlinked, or outside the active library.

**File**: `src/app.rs` (layout/source-level tests, if needed)

**Intent**: Guard the default search state and empty-state import action in the project’s existing lightweight frontend-test style.

**Contract**: The test asserts the search-grid/empty-state selectors and responsive CSS hooks, without pretending to exercise a browser runtime.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including encrypted catalogue migration, imported-only query, filter intersection, and preview containment coverage.
- `cargo tauri build` succeeds with the registered native command and Dioxus search view.

#### Manual Verification:

- Unlock a library containing imported image and video records; it opens directly to a full-window grid, and type/date filters return only matching imported media.
- Unlock an empty library; its full-window empty state explains the condition and Import media enters the existing safe import flow.
- Confirm a skipped item never appears, an unavailable/unsupported preview remains an identifiable card, and no original source media is changed.

**Implementation Note**: After automated verification passes, pause for human confirmation of the manual checks before Phase 2.

---

## Phase 2: Original-date search and tag suggestions

### Overview

Preserve both user-relevant date meanings for new imports and make tag filtering discoverable without changing the imported-only search boundary.

### Changes Required:

#### 1. Persist original discovered dates during review/import

**File**: `src-tauri/src/review.rs`

**Intent**: Capture the best available original date and its origin from the source while it is still available during an import decision, alongside the selected import date.

**Contract**: `import_review_item` records nullable `original_media_date` and `original_date_origin` for successful imports only. The existing selected date remains canonical for the managed destination and unchanged for skipped decisions.

**File**: `src-tauri/src/library.rs`

**Intent**: Extend the encrypted schema and transactional migration for the nullable original-date fields.

**Contract**: Existing items preserve their current data and receive null original-date fields; new libraries create the complete schema at the new version.

#### 2. Date-field selection and tag suggestion APIs

**File**: `src-tauri/src/search.rs`

**Intent**: Let search choose selected-import date or original-media date, and retrieve available normalized tags after a two-character prefix.

**Contract**: `SearchLibraryRequest` has a date-field enum plus optional inclusive range; `suggest_library_tags(TagSuggestionRequest)` accepts a normalized prefix of at least two characters, returns a deterministic bounded list of tags used by imported media only, and does not expose skipped-only tags. Multiple selected tags use AND semantics.

**File**: `src-tauri/src/lib.rs`

**Intent**: Register the tag-suggestion command alongside the search command.

**Contract**: `suggest_library_tags` uses the same authenticated catalogue boundary and structured errors as search.

#### 3. Refine the default search UI

**File**: `src/app.rs`

**Intent**: Add an original-vs-selected date selector and an accessible tag input that offers normalized tags after two typed characters.

**Contract**: Selected tag chips update the same search request using AND semantics; clearing a chip/filter re-runs the query. Original-date filtering clearly communicates that existing imported records may have no original date and therefore do not match that mode.

**File**: `assets/styles.css`

**Intent**: Style tag suggestions, selected chips, and the date-field chooser within the existing responsive form language.

**Contract**: Suggestions are visibly associated with the tag input, keyboard reachable, and remain usable in the narrow layout.

#### 4. Phase-two automated coverage

**File**: `src-tauri/src/review.rs` and `src-tauri/src/search.rs` (tests)

**Intent**: Verify date preservation and suggestion/query semantics across a reopen and schema upgrade.

**Contract**: Tests cover new import persistence of both dates; legacy null original dates; date-field/range behavior; two-character prefix threshold; imported-only suggestions; AND tag matching; and migration/reopen preservation.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including schema upgrade, original-date persistence, tag-prefix, and combined-filter coverage.
- `cargo tauri build` succeeds with both Tauri commands and the complete search UI.

#### Manual Verification:

- Import an item whose original discovered date differs from the selected import date; each date mode finds it only in the appropriate range.
- Type two characters into the tag field, select one or more suggestions, and verify only imported items carrying every selected tag remain.
- Reopen an existing library and verify prior items remain searchable by selected import date while the UI explains why they do not match original-date filters.

**Implementation Note**: After automated verification passes, pause for human confirmation of the manual checks before declaring the slice complete.

## Testing Strategy

### Unit Tests:

- Query imported records only, with optional date/type/tag intersections and stable result ordering.
- Validate managed-destination containment, readability, and symlink rejection before asset scoping.
- Exercise encrypted catalogue migration from the current version and persistence of both date fields across lock/reopen.
- Verify tag suggestions require two characters, are bounded/deterministic, and ignore skipped-only tags.

### Integration Tests:

- Set up a temporary protected library, import media, then issue search/suggestion requests through the native module seam.
- Confirm empty and cleanup-cleared catalogues return a valid empty result rather than an error.

### Manual Testing Steps:

1. Unlock an empty protected library and confirm its empty state uses the available application space before using Import media to complete a safe import.
2. Return to Library Search; verify imported images, supported videos, and unavailable-preview fallbacks are identifiable in the grid.
3. Apply type, selected-date, original-date, and multiple-tag filters; verify skipped/pending/source-only media never appears.

## Performance Considerations

The initial query targets a single local encrypted catalogue and bounded UI result set. Add indexes only for the chosen imported-decision/date/type access paths; do not introduce FTS, thumbnail generation, or eager media-file scanning. Preview validation happens only for returned records and never traverses source folders.

## Migration Notes

The catalogue format increases once for the phase-one search indexes and again for phase-two original-date columns, using the existing transaction, `schema_migrations`, and `library_identity` protocol. Existing items retain `effective_import_date`; original media date is null and deliberately not reconstructed from managed copies.

## References

- Product requirement: `context/foundation/prd.md:75-78`
- Roadmap slice and dependency boundary: `context/foundation/roadmap.md:114-125`
- Authenticated catalogue seam and versioned migration: `src-tauri/src/library.rs:302-338`, `src-tauri/src/library.rs:874-949`
- Import decision persistence and date discovery: `src-tauri/src/review.rs:279-301`, `src-tauri/src/review.rs:378-405`, `src-tauri/src/review.rs:512-522`
- Existing dynamic preview pattern: `src-tauri/src/review.rs:188-258`
- Existing unlocked-home and review rendering: `src/app.rs:880-970`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Default library search and empty state

#### Automated

- [ ] 1.1 Add the encrypted imported-media search query, schema/index migration, and safe preview boundary
- [ ] 1.2 Register the search command and implement the default Library Search view, filters, grid, and empty-state import action
- [ ] 1.3 Run `cargo test --workspace` for migration, query, containment, and UI-source coverage
- [ ] 1.4 Run `cargo tauri build`

#### Manual

- [ ] 1.5 Verify the default-after-unlock grid, filters, empty-state import action, preview fallbacks, and source safety

### Phase 2: Original-date search and tag suggestions

#### Automated

- [ ] 2.1 Persist original media dates and migrate existing catalogues safely
- [ ] 2.2 Add original/selected date modes, two-character imported-tag suggestions, and multi-tag AND filtering to the native/UI search flow
- [ ] 2.3 Run `cargo test --workspace` for date persistence, migration, suggestion, and combined-filter coverage
- [ ] 2.4 Run `cargo tauri build`

#### Manual

- [ ] 2.5 Verify both date modes, tag suggestions, AND filtering, and behavior for legacy imported records

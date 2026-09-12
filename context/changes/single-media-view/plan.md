# Single media view implementation plan

## Overview

Add a full-window media preview to managed-library search. Users will open a compact thumbnail, inspect its metadata, navigate the current result set, play videos, manage tags, reveal the managed copy in the platform file manager, and safely rotate or delete that managed copy.

## Current State Analysis

Library search already returns active imported records in a stable order, but its cards render extra file details and have no selection state. The Dioxus app has a reusable accessible overlay and review UI patterns, while the Rust layer has no candidate-scoped preview-detail or mutation commands. Imported copies live inside the protected library; original source media must remain untouched.

## Desired End State

Clicking a search thumbnail opens an accessible, full-window preview over the unchanged filtered result set. The user can inspect the item and edit its tags immediately, navigate without wrapping, recover from preview failures, rotate the managed image immediately, and send a managed copy to Trash only after explicit confirmation. Every filesystem action is authorized by an opaque candidate ID and revalidated against the unlocked managed library.

### Key Discoveries:

- `search_library` returns active imported items ordered by `effective_import_date DESC, candidate_id ASC`, making the frontend search result list the source of truth for preview position and navigation (`src-tauri/src/search.rs:116`, `src-tauri/src/search.rs:178`).
- The Rust search DTO already exposes `candidate_id`, but the frontend deserializer omits it (`src-tauri/src/search.rs:60`, `src/app.rs:304`); adding it is required before a selected item can invoke native operations.
- The comparison dialog is an accessible overlay with `aria-modal`, focus, and Escape handling that the preview can reuse (`src/app.rs:1674`).
- `safe_preview_url` already defends the asset protocol against missing files, symlinks, paths outside the managed root, and `.photo-handler` data (`src-tauri/src/search.rs:310`).
- Imported copies are made during review and the product guardrail prohibits deleting originals (`src-tauri/src/review.rs:454`, `context/foundation/prd.md`).

## What We're NOT Doing

- Editing, deleting, moving, or revealing original import-source media.
- Exact file selection in Finder/Explorer; this change opens the validated managed item's containing folder with the platform-native file manager.
- Rotation support for videos, HEIC, or any image codec that the existing image stack cannot safely decode and overwrite.
- A deleted-media history UI, undo control, ratings/favorites, export/share, or a separate frontend test framework.
- Changes to search filtering, ordering, or database schema.

## Implementation Approach

Keep the filtered `search_items` vector and an optional selected index in `App`; this preserves the searched-list context for the position indicator and left/right navigation. Add candidate-ID-only native commands in the search module, all built on one resolver that proves the item is an active imported record and that its managed path is a readable non-symlink under the active library. Deliver the read-only preview first, metadata/tag tools second, and managed-copy mutations last, so every phase can be manually exercised from the search UI.

## Critical Implementation Details

The native resolver must be the sole authority for every per-item action: it accepts a `candidate_id`, rechecks the active-import predicate, canonicalizes the managed file below the library root, and rejects symlinks and `.photo-handler`. Trash the managed file before clearing its `destination_path`; if catalogue update fails, preserve the stale record for an explicit recoverable error instead of risking deletion of another file. Rotation writes a same-directory temporary output and replaces the original managed copy only after encoding completes.

## Phase 1: Open and navigate media preview

### Overview

Make search thumbnails open a full-window, accessible, read-only preview with compact cards, navigation, playback, viewing controls, position, metadata, and clear loading/error behavior.

### Changes Required:

#### 1. Candidate-scoped preview detail and safety resolver

**File**: `src-tauri/src/search.rs`

**Intent**: Add a read-only media-details operation that provides the selected managed item's preview state, safe asset URL, tags, and technical metadata without exposing filesystem paths to the webview.

**Contract**: Introduce a private active-imported-item resolver keyed only by `candidate_id`; it must apply the current search predicate, validate canonical managed-file containment, and return structured unavailable reasons. Reuse or extract the review metadata reader so details include size, timestamps, dimensions, EXIF fields, and GPS when available.

#### 2. Native command registration

**File**: `src-tauri/src/lib.rs`

**Intent**: Expose the candidate-scoped media-details command to the Dioxus client using the established command wrapper and handler registration pattern.

**Contract**: The command accepts a request containing only `candidate_id` and returns the `search` module result/error type; add it to both a `#[tauri::command]` wrapper and `generate_handler!`.

#### 3. Preview state, overlay, and compact result cards

**File**: `src/app.rs`

**Intent**: Turn each search card into an accessible preview trigger and render an overlay that preserves the current result-list order and selected index.

**Contract**: Add `candidate_id` to `SearchLibraryItem`, App-owned optional preview index, detail/loading/error state, and zoom/view state. Remove `.media-card-details`; render an image or native `<video controls>` for the selected item; use the comparison dialog's modal semantics and handle `Esc`, `ArrowLeft`, and `ArrowRight` without intercepting text-entry controls. Disable previous/next at the ends and display `N of total`.

#### 4. Preview presentation and failure recovery styling

**File**: `assets/styles.css`

**Intent**: Style the viewport-filling preview, top toolbar, media canvas, expandable information panel, compact grid cards, and loading/unavailable states consistently with the existing application.

**Contract**: Extend the established overlay, media containment, and responsive grid styles; image controls must provide zoom in/out, fit-to-window, and actual-size behavior, while unsupported rotation actions are absent for video or unsupported-image detail states. Failure UI retains metadata/navigation, offers Retry for load/readability refreshes, and offers file-manager reveal where safe.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with unit coverage for candidate resolution, unavailable states, and out-of-root/symlink rejection.
- `cargo fmt --check` passes.

#### Manual Verification:

- A search result opens as a full-window preview; its original card has no filename, date, type, or tag details.
- `Esc` closes the preview; arrow keys navigate the filtered results without wrapping; the position indicator and disabled end controls are correct.
- Images honor zoom, fit, and actual-size controls; videos expose play/pause, timeline, volume, and duration controls.
- Missing, unreadable, unsupported, and browser-decode failures retain preview context and show actionable recovery UI.

**Implementation Note**: After completing this phase and all automated verification passes, pause for human confirmation that the manual preview checks succeeded before proceeding.

---

## Phase 2: Manage preview metadata and tags

### Overview

Make the information panel a persistent, immediately saved metadata workspace with tag autocomplete, trusted path-copy feedback, and platform-native folder reveal.

### Changes Required:

#### 1. Exact tag-set update operation

**File**: `src-tauri/src/search.rs`

**Intent**: Persist preview tag additions and removals for active imported records without reusing the pending-review decision flow.

**Contract**: Add a candidate-ID and tags request that normalizes whitespace/case, deduplicates the desired set, and transactionally replaces `candidate_tags` associations. It must retain the active-imported and safe-path validation boundary; autocomplete continues to use the existing active imported tag query.

#### 2. Trusted copy and reveal operations

**Files**: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`, `src-tauri/capabilities/default.json`

**Intent**: Copy only a validated managed path to the system clipboard and open that item's containing folder using the platform file manager.

**Contract**: Add candidate-ID-only native commands for copy and reveal. Use the Tauri v2-compatible clipboard facility and register only its minimum required capability; use the existing opener integration on the validated parent directory, labeled Finder on macOS and Explorer on Windows. Do not return an absolute path through the search result DTO.

#### 3. Preview information-panel controls

**File**: `src/app.rs`

**Intent**: Render full technical metadata and make the sidebar collapsible, with immediate tag persistence, autocomplete, duplicate prevention, copy confirmation, and platform-aware reveal action.

**Contract**: Reuse the existing `ReviewMetadata` formatting and tag-chip patterns. Update the selected detail and corresponding `search_items` entry only after a successful tag write; on failure restore the displayed tag set and present an error. If edits make the selected item fail the active search filters, refresh the existing search request and close the overlay only if its candidate no longer appears.

#### 4. Metadata-workspace styling

**File**: `assets/styles.css`

**Intent**: Provide an accessible compact/expanded sidebar, tag suggestions, pending/error feedback, and a temporary “Copied” state without breaking the preview layout at smaller widths.

**Contract**: Follow the existing focus-visible, chip, and sidebar conventions; actionable controls must stay reachable by keyboard in both sidebar states.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with tests for tag normalization, duplicate removal, exact replacement, locked/non-imported candidate rejection, and trusted reveal/copy path validation.
- `cargo fmt --check` passes.

#### Manual Verification:

- The information panel shows filename, type, size, dates, dimensions, camera, orientation, GPS when available, tags, and path actions; unavailable metadata is clearly labeled.
- The panel collapses/expands, tags autocomplete from current library tags, multiple tags can be added quickly, and duplicates cannot be created.
- Add/remove operations save immediately, update the search result state, and give an understandable error if persistence fails.
- Copy path shows “Copied”; Reveal opens the managed item's containing folder in Finder on macOS or Explorer on Windows.

**Implementation Note**: After completing this phase and all automated verification passes, pause for human confirmation that the manual metadata and tag checks succeeded before proceeding.

---

## Phase 3: Safely change managed media

### Overview

Add immediate managed-copy rotation and confirmed deletion, preserving original media and keeping preview navigation consistent after mutations.

### Changes Required:

#### 1. Confirmed managed-copy mutation commands

**File**: `src-tauri/src/search.rs`

**Intent**: Implement safe, candidate-scoped image rotation and confirmed Trash deletion.

**Contract**: Rotation accepts only `candidate_id` and direction; it immediately overwrites the supported managed image via a flushed same-directory temporary output, returns refreshed details with a cache-busting preview URL, and rejects video, HEIC, corrupt, or unsupported formats. Deletion includes `candidate_id` and `confirmed`; Rust rejects unconfirmed deletion and moves the validated managed copy to the OS Trash before setting its active imported decision's `destination_path` to `NULL`.

#### 2. Mutation command registration

**File**: `src-tauri/src/lib.rs`

**Intent**: Register rotate and confirmed-delete commands with the same request/response boundary as the preview details commands.

**Contract**: Commands expose no arbitrary path input and return structured errors that let the preview retain context after failure.

#### 3. Confirmation and post-mutation preview behavior

**File**: `src/app.rs`

**Intent**: Offer immediate rotation and confirmed delete toolbar controls only where supported, and reconcile the selected result after success.

**Contract**: Rotation immediately overwrites only the managed copy and refreshes its versioned image URL/cache state plus metadata after success. The delete dialog identifies the managed filename, removes the deleted ID from `search_items` only after native success, selects the next item at the same index, selects the previous item if the final item was deleted, and closes when no results remain.

#### 4. Destructive-action styling and accessible dialogs

**File**: `assets/styles.css`

**Intent**: Distinguish destructive controls and confirmations while preserving focus visibility, clear cancellation, and responsive preview behavior.

**Contract**: Confirmations use the existing dialog accessibility model and prevent duplicate submissions while the native command is in flight.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with tests for immediate managed-image rotation, unconfirmed deletion rejection, managed-root/symlink/missing-path rejection, Trash failure preserving catalogue state, successful deletion hiding the item, and supported-image rotation without partial overwrite.
- `cargo fmt --check` passes.

#### Manual Verification:

- Rotation buttons are absent for videos and unsupported images; supported-image rotation immediately changes only the managed copy and refreshes the open preview.
- Delete requires confirmation, moves only the managed copy to the OS Trash, leaves the original import source intact, updates the counter, and opens the next/previous result according to the agreed boundary behavior.
- Cancelling or failing either mutation preserves the preview and explains what happened.

**Implementation Note**: After completing this phase and all automated verification passes, pause for human confirmation that the manual destructive-operation checks succeeded.

## Testing Strategy

### Unit Tests:

- Candidate resolver rejects locked, non-imported, replaced, missing, symlinked, internal-state, and out-of-library records.
- Media details preserve best-effort metadata and return distinct unavailable/unsupported failures.
- Tag updates normalize, deduplicate, replace exact associations, and affect active tag suggestions correctly.
- Copy/reveal, rotation, and Trash deletion accept only validated active managed candidates; injected filesystem helpers cover external-operation failures.

### Integration Tests:

- Run `cargo test --workspace` for native command/module behavior; no browser test harness exists yet.

### Manual Testing Steps:

1. Start `cargo tauri dev`, search a mixed image/video managed library, and open cards from several filtered result sets.
2. Verify keyboard, toolbar, video, sidebar, metadata, tag, copy, reveal, loading, and failure flows in the desktop app.
3. Rotate a supported managed image and confirm its original source is unchanged; verify unsupported formats have no rotation controls.
4. Delete a managed item, confirm it is in OS Trash, verify navigation/counter updates, and confirm its original import-source file remains intact.

## Performance Considerations

Keep navigation in the existing in-memory filtered search vector; do not issue a native navigation query on each arrow press. Load rich metadata only for the selected item, refresh it after mutations, and retain the safe asset URL guard before exposing a preview.

## Migration Notes

No schema migration is required. Tag changes reuse `tags`/`candidate_tags`; deletion hides a trashed managed item by clearing `item_decisions.destination_path`, which existing search and tag queries already exclude. The catalogue intentionally preserves decision history without adding a deleted-history UI.

## References

- Requirements: `context/changes/single-media-view/requirments.md`
- Product guardrail: `context/foundation/prd.md`
- Search and safe asset access: `src-tauri/src/search.rs:60`, `src-tauri/src/search.rs:310`
- Existing review metadata/tag behavior: `src-tauri/src/review.rs:778`, `src-tauri/src/review.rs:1122`
- Existing modal and search UI: `src/app.rs:304`, `src/app.rs:1674`
- Existing cleanup-to-Trash behavior: `src-tauri/src/library.rs:300`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Open and navigate media preview

#### Automated

- [x] 1.1 Run Rust tests for candidate resolution and unavailable preview states — 0e48179
- [x] 1.2 Run `cargo fmt --check` — 0e48179

#### Manual

- [x] 1.3 Verify compact cards, full-window preview, keyboard navigation, and position indicators — 0e48179
- [x] 1.4 Verify image viewing controls, video playback controls, and recoverable preview failures — 0e48179

### Phase 2: Manage preview metadata and tags

#### Automated

- [x] 2.1 Run Rust tests for tag updates and trusted path actions — 502417f
- [x] 2.2 Run `cargo fmt --check` — 502417f

#### Manual

- [x] 2.3 Verify metadata panel, immediate tag editing, autocomplete, and duplicate prevention — 502417f
- [x] 2.4 Verify copy confirmation and platform-native managed-folder reveal — 502417f

### Phase 3: Safely change managed media

#### Automated

- [x] 3.1 Run Rust tests for immediate rotation, Trash deletion, and mutation failures — 178f6c6
- [x] 3.2 Run `cargo fmt --check` — 7ef4a6c

#### Manual

- [x] 3.3 Verify immediate managed-copy rotation, preview refresh, and unsupported-media controls — 178f6c6
- [x] 3.4 Verify Trash deletion, result navigation, and original-source preservation — 178f6c6

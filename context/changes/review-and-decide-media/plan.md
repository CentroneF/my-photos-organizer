# Review and decide media — Implementation Plan

## Overview

Add the first complete review loop for a selected import source. A person can recursively review supported photos and videos one at a time, preview them in the app, adjust the effective import date, add tags, and explicitly import or skip each item. Import copies a file into the managed library under `/<year>/<date>/` with a collision-safe unique name; it never moves, deletes, renames, or overwrites the original.

## Current State Analysis

The completed source-selection slice persists one selected folder and displays it on the library home, but does not enumerate its contents or create an import session. The encrypted SQLCipher catalogue has only schema-version and identity tables. Unlocking validates a password and immediately zeroes the database key, so no current command can safely read or write catalogue data after unlock. The UI has no media DTOs, review route, preview support, tag UI, or decision controls.

## Desired End State

After unlocking a library with an available import source, a person can start a review from the home screen. The app recursively identifies the supported files, safely excludes managed-library state, and opens a resumable one-item review experience. A reviewed file can be tagged, given an editable import date, imported by an atomic copy into the managed library, or skipped. Decisions survive restart; the person can resume the current source or choose a different folder. When no items remain, a completion state confirms that originals were left untouched.

### Key Discoveries:

- `src/app.rs:192` loads the selected source only after the protected library reaches `home`; its home view at `src/app.rs:588` offers only Choose/Change.
- `src-tauri/src/import_source.rs:47` currently validates and remembers a source path without enumerating it; S-03 is the first slice authorized to read source contents, but must preserve its non-mutating guarantee.
- `src-tauri/src/library.rs:318` validates the encrypted catalogue then clears the key. A process-local authenticated session is required before catalogue commands can operate without repeatedly sending the password.
- `src-tauri/src/library.rs:539` initializes only migration and library-identity tables, providing the migration seam for sessions, media, tags, and decisions.
- Tauri documents that dialog-selected paths are added to asset scopes, and the persisted-scope plugin can restore chosen asset scopes after restart; use this scoped route for native video/photo playback rather than granting a broad frontend filesystem capability. [Dialog reference](https://v2.tauri.app/reference/javascript/dialog/), [asset protocol scope](https://v2.tauri.app/fr/security/asset-protocol/)
- The roadmap assigns exact-duplicate, visual-similarity, and rich prior-history context to S-04; this slice persists minimal decisions only and must not surface those suggestions.

## What We're NOT Doing

- Moving, deleting, overwriting, or otherwise modifying source media; post-import source deletion remains out of scope.
- Exact hashing, duplicate detection, visual similarity, or suggestion UI (S-04).
- Library search, map/location features, cloud sync, sharing, or multi-user support (S-05 and post-MVP work).
- Arbitrary filesystem access from the frontend, background watching, or automatic re-import from changed sources.
- Date-folder organization beyond the selected `/<year>/<date>/` import destination, including user-configurable naming schemes.

## Implementation Approach

Keep filesystem mutation, media discovery, SQLCipher access, and destination naming in native Rust. Extend the library module with an application-managed, mutex-protected unlocked session containing the selected canonical library path and database key; establish it only after successful unlock/setup/recovery, and clear/zero it on explicit lock and process teardown. Add a versioned catalogue migration for review sessions, source-relative candidates, tags, tag joins, and minimal import/skip decisions.

The native review module will recursively enumerate a conservative documented list of common formats (`jpg`, `jpeg`, `png`, `webp`, `gif`, `heic`, `mp4`, `mov`, `m4v`, and `webm`), ignore unsupported files, and use source-relative paths plus metadata to resume an existing session. It will return bounded review DTOs and scoped asset URLs for native webview preview/playback. On import it resolves the chosen date from image metadata, otherwise file creation time, accepts the UI override, reserves a unique destination name, copies to a temporary sibling, atomically publishes it, then records the imported decision in one catalogue transaction. A skip records only the session candidate decision; it deliberately stores no content hash or cross-source matching data.

## Critical Implementation Details

Discovery must reject a source that is the managed library, contains it, or is contained by it before traversal, and must skip `.photo-handler` defensively. A session is resumed only when its remembered source remains available; choosing a different source leaves its prior session intact and makes the new one current. If a selected file disappears or becomes unreadable before a decision, retain its candidate and show a recoverable unavailable state rather than treating it as skipped.

The destination copy is a two-step publication boundary: copy bytes into a uniquely named temporary file in the final date directory, then rename it only after copying succeeds. Do not insert an imported decision until publication succeeds; if catalogue recording fails after publication, report the recovery state explicitly and never retry by overwriting an existing destination.

## Phase 1: Start and resume a safe review session

### Overview

Deliver a usable library-home path that starts or resumes a recursive, read-only review of a selected source and persists the candidate queue in the protected catalogue.

### Changes Required:

#### 1. Authenticated catalogue session and review schema

**Files**: `src-tauri/src/library.rs`, new `src-tauri/src/review.rs`, `src-tauri/src/lib.rs`

**Intent**: Make protected catalogue access available to review commands only while the library is unlocked, and create the durable records needed to resume a source review.

**Contract**: Establish a native process-local session after setup, unlock, open-existing, and password-recovery success; expose an explicit lock command that clears it. Advance the SQLCipher schema transactionally with records for an import session, candidates, tags, candidate tags, and an item decision. The schema must store canonical source identity plus source-relative candidate path and lightweight discovery metadata, while reserving no hash, duplicate, or similarity fields. Every review command requires the active session and returns a structured `library_locked` error if it is absent or no longer matches the selected library.

#### 2. Safe recursive discovery and session commands

**Files**: new `src-tauri/src/review.rs`, `src-tauri/src/import_source.rs`, `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`

**Intent**: Turn an available selected source into an explicit review session without changing any source content.

**Contract**: Add commands to start a source review, load the current/resumable review state, and select a different source through the existing picker flow. Discovery recursively reads only directories and documented common-media extensions, has deterministic ordering, excludes managed-library containment/overlap and `.photo-handler`, and records pending candidates transactionally. A new session is created only after preflight completes; the same available source resumes its unfinished session rather than duplicating candidates. Configure only the scoped asset access/persistence required for previewing user-selected sources after restart; do not add broad frontend file-reading permission.

#### 3. Library-home review entry state

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Give a user a clear start/resume/change-folder decision immediately after protected-library unlock.

**Contract**: When the selected source is ready, render Start review when no unfinished session exists and Resume review when one does. Retain the existing Change-folder action; changing a folder returns the user to home and does not silently discard another source's session. Render loading, empty/unsupported-source, unavailable-source, and native error states accessibly, always stating that discovery reads files but does not modify originals.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including encrypted-catalogue migration, active-session, lock, deterministic-recursive-discovery, overlap-rejection, and no-source-mutation tests.
- `cargo check --workspace` succeeds with the new review commands and scoped preview configuration.
- Tests prove that starting, resuming, changing, and failing a review session never changes source-file bytes, names, paths, or directory entries.

#### Manual Verification:

- After unlocking, a person can start a review of a selected folder containing supported files in subfolders and see a loading then review entry state.
- Closing/relaunching, unlocking, and returning home offers Resume review for the unfinished source; changing the source instead leaves the first session resumable.
- Empty, unsupported, unavailable, same-as-library, containing-library, and inside-library sources produce clear recoverable states without source mutation.

**Implementation Note**: After automated checks pass, pause for human confirmation of the desktop session-start and resume behavior before continuing.

---

## Phase 2: Review, tag, import, and skip media

### Overview

Deliver the complete one-item-at-a-time decision experience, including native previews, editable import dates, durable tags/skips, and safe date-folder imports.

### Changes Required:

#### 1. Review-item, preview, and decision commands

**Files**: new `src-tauri/src/review.rs`, `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`

**Intent**: Provide native operations for loading the next pending candidate, rendering it safely, and committing an explicit user decision.

**Contract**: Return an item DTO with source-relative display path, filename, media type, effective import date and its origin, editable-date validation, current tags, and a scoped preview URL suitable for an image or HTML video element. Persist normalized non-empty user tags and their joins with the decision. Skip changes only the candidate decision and does not calculate or retain hashes. Import accepts the user-approved date and tags, verifies the candidate remains readable, then follows the atomic-copy contract into `<managed library>/<year>/<date>/`; a collision receives a generated unique filename and never replaces an existing file. Persist the imported record only after file publication and return a specific recoverable error for any post-copy catalogue failure.

#### 2. Focused review UI

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Let the user understand and decide one item at a time with no implicit import, skip, or date change.

**Contract**: Replace the review-entry state with a focused card showing an image preview or playable in-app video, filename/path context, tag entry/removal, the metadata-derived date or creation-date fallback, and a date field the user can change before importing. Import and Skip are separate deliberate controls; both show busy/error feedback and advance only after a successful native response. Display the resulting managed destination after import. If a candidate disappeared, is unsupported by the webview, or cannot be previewed, preserve its identity and offer an error/metadata fallback without auto-skipping it.

#### 3. Media and copy safety tests

**Files**: tests in `src-tauri/src/review.rs`, optionally `src-tauri/src/library.rs`

**Intent**: Make the import guardrails and decision persistence independently verifiable.

**Contract**: Use temporary source/library folders and a correctly keyed catalogue to test media filtering, tag normalization, metadata-date then creation-date selection, date override validation, skip persistence across session reopen, unique collision naming, exact copy bytes, source preservation, unavailable-before-decision handling, interrupted-copy cleanup, and catalogue-write failure behavior. Add a browser-compatible sample photo and video fixture only if needed for manual preview verification; do not commit personal media.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including encrypted decision persistence and atomic-copy failure/collision coverage.
- `cargo check --workspace` succeeds and the frontend DTOs stay synchronized with native command contracts.
- Tests demonstrate source media remains byte-for-byte unchanged after imports, skips, validation failures, and destination collisions.

#### Manual Verification:

- A person can view a supported photo and play a supported video in the desktop review UI, add/remove tags, edit the import date, and explicitly choose Import or Skip.
- Importing creates one copied file in the expected `<year>/<date>/` folder with a unique name when necessary; the original file remains unchanged at its source path.
- Skipped items remain skipped after app restart, while a file unavailable just before a decision displays a recoverable error rather than being silently decided.

**Implementation Note**: After automated checks pass, pause for human confirmation of actual photo/video preview, import, and source-preservation behavior before continuing.

---

## Phase 3: Finish the review loop and harden verification

### Overview

Complete the user-visible review lifecycle with an honest completion state and regression coverage for restart, incomplete work, and safe recovery.

### Changes Required:

#### 1. Completion and recovery states

**Files**: `src/app.rs`, `assets/styles.css`, new or updated `src-tauri/src/review.rs`

**Intent**: Let a person know when every discovered item has received a decision and return them safely to library home actions.

**Contract**: When no pending candidate remains, return a completion DTO with imported/skipped counts and render a completion screen that confirms originals were not deleted or moved. Offer Resume/Review another folder via home, but no delete-original action. A catalogue/session failure returns a state that retains the current item/session context and directs the user to retry or lock/reopen, without exposing database keys or passwords.

#### 2. Lifecycle regression coverage

**Files**: tests in `src-tauri/src/review.rs`, `src-tauri/src/library.rs`, optionally `src/app.rs`

**Intent**: Verify the complete vertical flow is durable and that failures never turn into unrequested media mutations.

**Contract**: Cover completion counts, restart/resume with pending and completed sessions, switching sources, lock rejection of review commands, failure to persist a decision, and unsuccessful destination publication. Add a static/manual verification checklist for macOS and Windows codec support; document a browser playback failure as an item-level fallback, not an automatic decision.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes for the full start → decide → restart/resume → complete lifecycle.
- `cargo check --workspace` succeeds, and a code search verifies no S-03 source path invokes removal, rename, or overwrite behavior.
- Tests cover failure paths with no partial published import and no unintended source change.

#### Manual Verification:

- Completing all items shows accurate import/skip counts and explicitly confirms originals remain untouched.
- A user can lock/reopen, resume an unfinished review, or select another folder from the finished state without losing durable decisions.
- On both supported desktop platforms, manually verify common photo/video playback or the documented item-level fallback for a codec the embedded webview cannot play.

**Implementation Note**: After automated checks pass, pause for human confirmation of the full desktop flow before considering the change complete.

## Testing Strategy

### Unit Tests:

- Transactional schema migration and encrypted catalogue access only through an active unlocked session.
- Recursive candidate discovery, stable ordering, common-format filtering, and managed-library overlap rejection.
- Metadata date extraction, creation-date fallback, user date validation, and tag normalization.
- Durable skip/import records, restart/resume behavior, unique destination naming, atomic copy, and failure cleanup.

### Integration Tests:

- Tauri commands expose matching serializable DTOs for session status, review item, decision result, and structured errors.
- Dialog-selected/persisted asset scope permits the selected item preview without granting the frontend arbitrary filesystem read access.
- Library lock rejects subsequent review commands until the next successful unlock.

### Manual Testing Steps:

1. Create/unlock a protected library, choose a nested source with sample common photos and videos, and start review.
2. Preview a photo and video, add tags, change an import date, import a file, and verify its copied date-folder destination plus unchanged source bytes.
3. Skip a second item, restart/unlock, resume, and verify that it is not presented again while the next pending item is.
4. Exercise a filename collision, moved candidate, unavailable source, and source/library containment attempt; verify clear errors with no unintended files changed.
5. Complete the review and verify counts, no delete-original affordance, and the ability to start/resume another source.

## Performance Considerations

Discovery is an explicit user action and must stream or page native results rather than return an unbounded file list to the webview. Persist only bounded item metadata needed for the next review card; load previews on demand. Copy happens only after an explicit Import action and should report progress for large video files without blocking other UI feedback. The first version does not hash every file, avoiding a full source-wide I/O pass; S-04 can introduce hashing with its duplicate/history design.

## Migration Notes

Advance the catalogue schema version transactionally from the existing version-1 identity schema. Existing protected libraries have no review records, so their first available source starts a fresh session. Existing import-source pointers remain app-local selection state and are never converted into decisions. Migration failure must preserve the existing encrypted catalogue and surface a recoverable error rather than attempting a partial reset.

## References

- Product requirements: `context/foundation/prd.md:64`
- Original-media guardrail: `context/foundation/prd.md:97`
- S-03 scope and S-04 boundary: `context/foundation/roadmap.md:89`
- Completed source-selection contract: `context/changes/choose-import-folder/plan.md:5`
- Existing library-home UI: `src/app.rs:588`
- Source validation/pointer: `src-tauri/src/import_source.rs:47`
- Encrypted catalogue setup/migration seam: `src-tauri/src/library.rs:539`
- Current unlock key lifetime: `src-tauri/src/library.rs:318`
- Scoped local asset guidance: [Tauri dialog reference](https://v2.tauri.app/reference/javascript/dialog/), [Tauri asset protocol scope](https://v2.tauri.app/fr/security/asset-protocol/)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Start and resume a safe review session

#### Automated

- [x] 1.1 Add the authenticated catalogue session and transactional review schema
- [x] 1.2 Add safe recursive discovery and review-session commands
- [x] 1.3 Add the library-home start, resume, and change-folder review entry states
- [x] 1.4 Verify workspace tests, compilation, source preservation, and overlap rejection

#### Manual

- [x] 1.5 Confirm desktop session start, restart resume, folder change, and safe invalid-source states

### Phase 2: Review, tag, import, and skip media

#### Automated

- [ ] 2.1 Add native review-item, preview, tag, skip, and atomic date-folder import commands
- [ ] 2.2 Add the focused photo/video review UI with editable date and explicit decisions
- [ ] 2.3 Verify workspace tests, compilation, encrypted decision persistence, and copy/source safety

#### Manual

- [ ] 2.4 Confirm desktop image/video preview, tagging, date override, import, skip, collision naming, and source preservation

### Phase 3: Finish the review loop and harden verification

#### Automated

- [ ] 3.1 Add completion/recovery state and lifecycle regression coverage
- [ ] 3.2 Verify workspace tests, compilation, full lifecycle, and no partial-import/source-mutation failures

#### Manual

- [ ] 3.3 Confirm completion counts, lock/reopen resume, folder switching, and platform playback fallback behavior

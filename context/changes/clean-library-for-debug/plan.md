# Clean library for debug — Implementation Plan

## Overview

Add a password-confirmed library-clean action for debug use. It will move Photo Handler's top-level date-organized managed-media folders to the operating system Trash, clear the remembered import folder and catalogue review data only after every eligible folder is removed, and retain the protected library setup.

## Current State Analysis

The protected library is an otherwise user-owned root folder containing `.photo-handler/library.json` and an encrypted `.photo-handler/catalogue.db`. The catalogue's version-3 schema stores review sessions, candidates, tags, tag links, and import/skip decisions; imported-copy destinations are recorded in `item_decisions.destination_path`. Imported copies currently publish directly under `<library>/<YYYY>/<YYYY-MM-DD>/`, while an interrupted import can leave an unrecorded copy in that layout.

The home screen has no destructive maintenance actions. It can lock the active session, select an import source, and start a review. The remembered import source is an app-data pointer separate from the protected library identity and selected-library pointer.

## Desired End State

From the unlocked library home, a person can open a clearly labelled danger-zone flow, enter the current library password, and explicitly clean the library. The app moves every eligible top-level managed-media year folder to the native Trash/Recycle Bin, removes all review/media metadata and the remembered import source only when all eligible folders were moved, then returns to the still-unlocked library home.

If any folder cannot be moved to Trash, the app reports a retryable partial cleanup and retains the entire catalogue so it can identify the remaining media later. The permanent protected setup—marker, encrypted database, password/recovery wraps, migration/identity rows, and selected-library pointer—remains valid and unlockable throughout. Original import-source media and all non-date content in the library root remain unchanged.

### Key Discoveries

- `.photo-handler/library.json` and `.photo-handler/catalogue.db` define the protected library state in `src-tauri/src/library.rs:16-21`; the authenticated session is the native-only authority for opening the encrypted catalogue in `src-tauri/src/library.rs:182-218`.
- The current schema has setup-only `schema_migrations` and `library_identity` rows plus all review data in five review tables in `src-tauri/src/library.rs:618`.
- `publish_copy` creates copied media directly at `<library>/<YYYY>/<YYYY-MM-DD>/` in `src-tauri/src/review.rs:595-675`; an earlier failure may leave an unrecorded copied file (`src-tauri/src/review.rs:286-294`).
- Source selection is persisted independently as `selected-import-source.json` in `src-tauri/src/import_source.rs:9-10` and `:144-176`.
- The Rust `trash` crate provides native Trash/Recycle Bin support for macOS and Windows. [trash documentation](https://docs.rs/crate/trash/latest)

## What We're NOT Doing

- Deleting, moving, renaming, or otherwise changing any original import-source media.
- Emptying the operating system Trash/Recycle Bin, restoring trashed content, or promising recovery after the OS Trash is emptied.
- Deleting `.photo-handler/`, resetting the password/recovery setup, removing the selected-library pointer, or recreating the library.
- Deleting arbitrary library-root files or folders: only top-level four-digit year folders whose direct child folders use the `YYYY-MM-DD` layout are cleanup targets.
- Adding media search, duplicate detection, background cleanup, cloud synchronization, or a general-purpose file manager.

## Implementation Approach

Keep the cleanup authority in native Rust. A new Tauri command will require both the active unlocked session and a fresh verification of the current password before it inspects the library root. It will classify eligible top-level managed-media year folders by the established layout, move each folder to the OS Trash, and only when every target is gone use one SQL transaction to clear all review tables and remove the selected import-source pointer.

The command is deliberately all-or-retry at the metadata boundary. If any folder cannot be trashed, it does not clear catalogue rows or the import-source pointer; some folders may already be in Trash, but the retained records let a later confirmed retry finish safely. The Dioxus home screen will provide an explicit password form, cancel path, busy state, success return, and retryable error without granting frontend filesystem access.

## Critical Implementation Details

The cleanup scan must inspect only direct children of the unlocked library root and must never follow a candidate through an unvalidated path. `.photo-handler` and every nonmatching root entry are excluded. Move whole eligible year folders to Trash rather than deleting individual files, so unrecorded copies from failed imports are covered and a folder-level Trash failure remains a clear retry unit.

## Phase 1: Password-confirmed library clean

### Overview

Deliver a complete debug-clean workflow that a user can run from the unlocked desktop home: explicitly re-enter the library password, move eligible managed copies to native Trash, wipe all review metadata and the remembered source after complete success, and stay unlocked.

### Changes Required

#### 1. Native cleanup command and protected-state helpers

**Files**: `src-tauri/Cargo.toml`, `src-tauri/src/library.rs`, `src-tauri/src/import_source.rs`, `src-tauri/src/lib.rs`

**Intent**: Make native Rust the sole authority for password re-verification, managed-folder classification, OS Trash operations, catalogue deletion, and source-pointer removal.

**Contract**: Add a pinned `trash` dependency and serializable `clean_library` request/result/error DTOs. The command requires an active authenticated session plus the current password, verifies that password against the existing library marker without replacing the active session, and clears temporary sensitive material. It scans only direct root entries matching a four-digit year with direct `YYYY-MM-DD` directory children, excludes `.photo-handler` and all nonmatching entries, and invokes the native Trash API on each eligible year folder. On complete success, one catalogue transaction deletes `candidate_tags`, `tags`, `item_decisions`, `review_candidates`, and `review_sessions` while preserving `schema_migrations` and `library_identity`, then removes `selected-import-source.json`. Register the command in the existing Tauri invoke handler.

#### 2. Explicit danger-zone confirmation UI

**File**: `src/app.rs`

**Intent**: Give an unlocked user a deliberate, understandable path to clean debug media without implying that source originals or protected setup will be removed.

**Contract**: Add home-screen danger-zone entry and a clean-confirmation state with a current-password field, unambiguous text that managed date folders go to OS Trash, and text that sources, `.photo-handler`, and unrelated library-root content are preserved. Serialize the password to `clean_library`, disable duplicate submissions while busy, clear the entered password on success/cancel/error handling, refresh the in-memory import-source and review state after success, remain on home unlocked, and render stable command errors through the existing `command_error` path.

#### 3. Complete-clean safety coverage

**Files**: tests in `src-tauri/src/library.rs`, `src-tauri/src/import_source.rs`, and/or a dedicated native cleanup module test section

**Intent**: Prove that successful cleanup removes only the intended managed media and review data while preserving a usable protected library and all originals.

**Contract**: Use temporary libraries and source folders to test locked-session rejection, incorrect-password rejection, a successful date-folder Trash cleanup, removal of all five review tables and selected import-source pointer, and preserved marker/database/schema/identity/unlockability. Assert that source files, `.photo-handler`, and nonmatching root files/directories are unchanged; include an unrecorded file inside an eligible date folder to prove orphaned copied media is sent to Trash with its owning year directory.

### Success Criteria

#### Automated Verification

- `cargo test --workspace` passes, including password, folder-classification, successful-clean, source-preservation, and protected-setup preservation tests.
- `cargo check --workspace` succeeds with the macOS and Windows Trash dependency available to the Tauri crate.
- Tests assert that only matching top-level managed date folders are passed to the Trash integration; `.photo-handler`, nonmatching root content, and source media are never selected.

#### Manual Verification

- From an unlocked desktop library with imported copies, the user opens the danger zone, supplies the current password, and sees copied date folders disappear from the library while the OS Trash contains them.
- After success, the app remains at unlocked library home, asks for an import folder again, and can later be locked and unlocked with the same password.
- Cancelling, entering a wrong password, or inspecting a library with unrelated root content leaves all media and catalogue state unchanged.

**Implementation Note**: After automated verification passes, pause for manual confirmation before Phase 2.

---

## Phase 2: Partial-cleanup recovery and hardening

### Overview

Make interrupted or partially failing cleanup explicit and safe to retry, without discarding the catalogue history needed to identify still-present managed media.

### Changes Required

#### 1. Retryable partial-cleanup result

**Files**: `src-tauri/src/library.rs`, `src-tauri/src/lib.rs`, `src/app.rs`

**Intent**: Preserve the ability to retry whenever one or more eligible year folders cannot be moved to the OS Trash.

**Contract**: Process each validated target independently and return a structured `cleanup_incomplete` error/result identifying only user-safe failure details. If any Trash operation fails, do not clear any review tables or the import-source pointer; retain all database information, including records for folders already sent to Trash, until a later invocation finds no eligible year folders and can complete the metadata transaction. The UI keeps the library unlocked, shows that cleanup is incomplete and retryable, clears the confirmation password, and allows a new password-confirmed retry.

#### 2. Failure-path and retry regression tests

**Files**: tests in `src-tauri/src/library.rs` and/or a dedicated cleanup module test section; `src/app.rs` only if frontend tests are introduced with an established test harness

**Intent**: Demonstrate that a partial operation never turns into silent metadata loss or broad filesystem deletion.

**Contract**: Introduce an injectable Trash-operation seam suitable for deterministic native tests. Cover a simulated failure after at least one eligible folder succeeds, asserting that review tables and the import-source pointer remain intact, failed folders remain in the library, unrelated/source files are untouched, and a subsequent successful retry clears metadata and source selection. Also test empty-library cleanup, malformed/non-date date-like paths, symlink/path-validation rejection, and stale/missing previously recorded destinations without ever escalating to arbitrary root deletion.

### Success Criteria

#### Automated Verification

- `cargo test --workspace` passes with deterministic partial-failure and successful-retry coverage.
- Tests prove a partial Trash failure retains all catalogue review data and the selected import-source pointer until a fully successful retry.
- Tests prove no cleanup path can target `.photo-handler`, source files, or a root entry outside the strict managed date-folder layout.

#### Manual Verification

- A forced or permission-induced Trash failure shows a clear retryable error, leaves the library unlocked, and does not present cleanup as complete.
- Retrying after the failure condition is resolved moves the remaining managed folders to Trash, clears review history/source selection, and leaves protected setup reusable.

**Implementation Note**: After automated verification passes, pause for manual confirmation before marking the change complete.

## Testing Strategy

### Unit Tests

- Strict classification accepts only direct `YYYY/YYYY-MM-DD` directory layouts and excludes `.photo-handler`, loose files, mixed-content year folders, symlinks, and non-date paths.
- A correct current password authorizes cleanup; locked and incorrect-password requests change nothing.
- A complete cleanup moves eligible folders through the Trash abstraction, clears the five review tables and remembered source, and preserves setup records and unlockability.
- A partial Trash failure retains all metadata and supports a later successful retry.

### Integration Tests

- Tauri command registration and DTO serialization return stable success, password, locked, invalid-target, and incomplete-cleanup outcomes.
- The existing review/import lifecycle can start again after a completed cleanup and a newly selected source folder.

### Manual Testing Steps

1. Set up/unlock a disposable library, import media from a separate source, and place unrelated root content beside the managed date folders.
2. Use the home danger zone, re-enter the library password, confirm media is in the OS Trash, and verify originals, `.photo-handler`, unrelated root content, and the protected-library login still work.
3. Confirm that no import source is remembered and a new source can begin a clean review after cleanup.
4. Induce a Trash failure for one eligible folder, confirm the retryable state and retained catalogue behavior, then resolve it and retry to successful completion.

## Performance Considerations

The cleanup is an explicit, foreground-only maintenance action. It scans only direct library-root entries and delegates whole-folder movement to the operating system Trash, avoiding unbounded recursive application-side deletion. The UI stays busy until the command completes to prevent concurrent review/import actions against changing managed media.

## Migration Notes

No schema migration is required: cleanup removes only mutable review data from the current version-3 tables and leaves `schema_migrations` and `library_identity` unchanged. The feature depends on the established root date-folder layout; it intentionally does not infer ownership from arbitrary files or database destination paths during destructive selection.

## References

- Product guardrail: `context/foundation/prd.md:38-41`, `context/foundation/prd.md:97-101`
- Protected-state and authenticated catalogue access: `src-tauri/src/library.rs:16-21`, `src-tauri/src/library.rs:142-218`, `src-tauri/src/library.rs:618`
- Managed-copy layout and failure state: `src-tauri/src/review.rs:286-294`, `src-tauri/src/review.rs:595-675`
- Import-source pointer: `src-tauri/src/import_source.rs:9-10`, `src-tauri/src/import_source.rs:99-176`
- Home UI: `src/app.rs:815-848`
- OS Trash API: [trash crate documentation](https://docs.rs/crate/trash/latest)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles.

### Phase 1: Password-confirmed library clean

#### Automated

- [x] 1.1 Add native password-confirmed cleanup, Trash integration, metadata/source-pointer clearing, and command registration. — 31e5c8a
- [x] 1.2 Add the unlocked-home danger-zone confirmation flow and cleanup state refresh. — 31e5c8a
- [x] 1.3 Verify workspace tests, compilation, strict target selection, and setup/source preservation. — 31e5c8a

#### Manual

- [x] 1.4 Confirm a complete password-confirmed desktop cleanup sends managed copies to Trash and preserves an unlocked, reusable library. — 31e5c8a

### Phase 2: Partial-cleanup recovery and hardening

#### Automated

- [x] 2.1 Add retryable incomplete-cleanup behavior that retains metadata and import-source selection until all targets are trashed.
- [x] 2.2 Add deterministic partial-failure, retry, malformed-path, and source/unrelated-content regression coverage.
- [x] 2.3 Verify workspace tests and compilation for complete and partial cleanup paths.

#### Manual

- [x] 2.4 Confirm the desktop retry flow after a Trash failure preserves state until a later successful cleanup.

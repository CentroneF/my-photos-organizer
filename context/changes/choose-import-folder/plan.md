# Choose import folder — Implementation Plan

## Overview

Add the first post-unlock library-home experience, where a person can choose a separate folder as the source for a future import, see that selection after restarting, change it at any time, and receive a clear unavailable state if it has moved. The source selection must not scan, copy, move, delete, or otherwise modify media or source-folder contents.

## Current State Analysis

The protected-library flow is complete: it creates or unlocks a fixed managed-library location and then ends on a success screen. Its existing folder picker is intentionally unsuitable for an import source because it inspects a folder as a library and rejects normal non-empty directories. The app already has native dialog support, a serializable Tauri-command convention, and an atomic JSON app-settings pointer pattern that can safely remember one path across application restarts.

## Desired End State

After a protected library is created, unlocked, or recovered, the user reaches a library home screen. They can select a separate folder using the native directory picker; Photo Handler remembers its path locally and displays it as the selected import source after a restart. If it no longer exists or is no longer a directory, the app shows the remembered path as unavailable and offers a clear change action. Selecting the protected managed-library directory itself is rejected. No source media is inspected, indexed, copied, moved, deleted, or recorded in the catalogue.

### Key Discoveries:

- `src/app.rs:172` uses `plugin:dialog|open` only to select a managed-library directory, then routes it through the library-specific inspection command.
- `src-tauri/src/library.rs:197` rejects a non-empty folder unless it is a recognized Photo Handler library, which is the opposite of the intended import-source behavior.
- `src-tauri/src/library.rs:238` writes the remembered managed-library location through a temporary JSON file followed by rename; use the same durable app-settings pattern for the import-source pointer.
- `src-tauri/src/lib.rs:121` already registers the dialog plugin and command handler seam; `src-tauri/capabilities/default.json:6` already grants `dialog:allow-open`.
- `context/foundation/roadmap.md:77` defines this as source selection only; review, tagging, import/skip decisions, and media handling remain in S-03.

## What We're NOT Doing

- Scanning, enumerating, filtering, classifying, hashing, previewing, or validating the media contents of an import source.
- Copying, moving, deleting, renaming, or changing any source media or source-folder content.
- Creating catalogue tables, import-session records, or a durable review queue.
- Implementing review, tagging, import/skip decisions, duplicate detection, or source-history management beyond one latest selected path.
- Adding dependencies, dialog capabilities, cloud services, or managed-library schema migrations.

## Implementation Approach

Keep the managed-library and import-source responsibilities distinct. Add a focused native import-source module and commands that receive the app data directory, validate a selected directory without reading its contents, compare its canonical location with the remembered managed-library root, and atomically persist a separate latest-source JSON pointer. The Dioxus root component will add library-home state and distinct signals/DTOs for import-source data, invoking the existing native directory picker and new commands only after the library is ready.

## Critical Implementation Details

Source validation is metadata-only: it confirms the selected path is an available directory and canonicalizes it solely to prevent the managed-library root from being selected through an alias. It must not call `read_dir` or perform any media discovery. A stale remembered source remains on disk as a pointer and is reported to the UI; it is not silently cleared.

## Phase 1: Choose and remember an import source

### Overview

Deliver the complete desktop flow from a newly ready library to a safely selected, persisted, and changeable import-source folder, with native safety tests and clear UI feedback.

### Changes Required:

#### 1. Native import-source selection and persistence

**Files**: new `src-tauri/src/import_source.rs`, `src-tauri/src/lib.rs`

**Intent**: Introduce a native boundary that owns source-path validation and app-local persistence, keeping all source-folder handling non-mutating and separate from protected-library setup.

**Contract**: Expose serializable request/result/error DTOs and register commands to save a selected source and retrieve its remembered state. The save command accepts one folder path, requires an available directory, resolves it only for equivalence testing, rejects the canonical managed-library root, then writes the latest source pointer atomically under the app data directory. The read command returns a stable `missing`, `ready`, or `stale` state plus the remembered path when present. Neither operation reads directory entries or writes anywhere outside app-local settings.

#### 2. Library-home and source-picker UI

**File**: `src/app.rs`

**Intent**: Replace the terminal protected-library success state with the ongoing library home that exposes the import-source choice and makes the persisted source state visible and changeable.

**Contract**: Maintain import-source state separately from the managed-library `folder` signal. When setup, unlock, or recovery succeeds, render the library home and load the remembered source state through the native command. A primary “Choose import folder” / “Change folder” action invokes the existing native directory picker with import-specific copy, persists the returned path through the new command, and renders the ready or stale source state. Picker cancellation keeps the prior selection unchanged; command failures render the existing accessible error treatment. The UI states plainly that choosing a source does not yet scan or modify files.

#### 3. Safety-focused native tests and verification

**Files**: new tests in `src-tauri/src/import_source.rs`, optionally `src-tauri/src/lib.rs`

**Intent**: Make source-selection safety and restart behavior repeatable without needing a live desktop window.

**Contract**: Use temporary directories to test successful pointer persistence and reload, a missing pointer, a remembered source that later becomes unavailable, rejection of the managed-library directory including canonical aliases, and rejected invalid selections. Assert that each source directory's contents remain byte-for-byte unchanged and that rejected selections do not replace an existing valid pointer.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including temporary-directory tests for persisted, stale, invalid, and same-as-library source paths.
- `cargo check --workspace` succeeds with the new commands registered and frontend DTOs compiling.
- Tests demonstrate that source-selection operations never enumerate, alter, or create files in the selected source directory.

#### Manual Verification:

- In the desktop UI, completing setup, unlocking, or password recovery leads to a library home with a “Choose import folder” action.
- Selecting a normal non-empty photo/video folder displays it as the import source; restarting the app, unlocking the library, and returning home shows the same source with a “Change” action.
- Selecting the protected managed-library folder shows a clear rejection, and a moved/unavailable remembered source is labelled unavailable while its change action remains available.
- Inspecting the chosen source after selection confirms no files or folders were added, changed, moved, or removed.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before considering the change complete. The corresponding checkbox items live in `## Progress`.

---

## Testing Strategy

### Unit Tests:

- Save and reload an available source path through the app-local pointer format.
- Return `missing` when no pointer exists and `stale` when a persisted path is no longer an available directory.
- Reject a selected path that canonicalizes to the managed-library root, including a symlink/alias case where the platform supports it.
- Preserve an existing source pointer and all selected-source contents when validation or persistence fails.

### Integration Tests:

- Native command DTOs return the declared ready/missing/stale states and structured errors for invalid or overlapping locations.
- Command registration remains reachable through the existing Tauri invoke handler.

### Manual Testing Steps:

1. Create or unlock a protected library and confirm the new library home appears.
2. Select a non-empty folder containing sample media and confirm the app records only its path, with no source-content change.
3. Restart, unlock the same library, and confirm the remembered source and a working change action appear.
4. Select the managed-library folder and confirm rejection; then move or make the remembered source unavailable and confirm the stale state and replacement path action.

## Performance Considerations

Path validation is limited to filesystem metadata and canonical-path resolution on explicit selection or home-state restoration. It intentionally does not traverse source directories, so its cost does not grow with media-library size.

## Migration Notes

No catalogue migration is required. The latest import-source pointer is new, app-local, and optional: older installations without it return the `missing` state. A stale pointer is retained for user context and can be replaced through the UI.

## References

- Product requirement: `context/foundation/prd.md:62`
- Source-selection scope and safety risk: `context/foundation/roadmap.md:77`
- Product no-deletion guardrail: `context/foundation/prd.md:99`
- Existing dialog invocation: `src/app.rs:172`
- Existing command registration: `src-tauri/src/lib.rs:121`
- Managed-library-only inspection: `src-tauri/src/library.rs:197`
- Atomic app-settings pointer precedent: `src-tauri/src/library.rs:238`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles.

### Phase 1: Choose and remember an import source

#### Automated

- [x] 1.1 Add native import-source validation, persistence, commands, and safety tests — 09d1945
- [x] 1.2 Add the library-home import-source picker and remembered/stale UI states — 09d1945
- [x] 1.3 Verify workspace tests, compilation, and no-source-mutation guarantees — 09d1945

#### Manual

- [x] 1.4 Confirm selection, restart persistence, safe managed-library rejection, and stale-source recovery in the desktop UI — 09d1945

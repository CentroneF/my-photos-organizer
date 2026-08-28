# Review and decide media UX changes — Implementation Plan

## Overview

Refine Photo Handler's post-library-selection experience into a centered, full-window workspace. The plan removes the onboarding chrome and manual locking controls, makes library actions easier to reach, moves destructive cleanup into its own in-app page, and ensures a review card remains usable without vertical page scrolling.

## Current State Analysis

The application always renders a two-column shell with a blue brand panel and a numbered two-step progress row. Once a library is selected, this shell remains around setup, unlock, home, review, and cleanup, although the user is no longer in an onboarding flow. The protected-library path and its home actions are in one view; cleanup confirmation is conditionally expanded inline; and manual lock actions appear on both home and completion. Review previews use a fixed height cap, so the whole card can exceed the available viewport.

## Desired End State

After selecting a library path, every library-related view uses a centered, single-column full-window workspace with no blue side panel or numbered step indicators. From home, the user can open the active library's contents in Finder/Explorer and enter a separate danger-zone page directly below the protected path. The review card scales to the viewport so its preview, fields, and Import/Skip actions are all accessible without scrolling.

### Key Discoveries:

- The global shell, sidebar, numbered progress row, and all view-state rendering are in `src/app.rs:779` and `src/app.rs:798`.
- The two-column layout, sidebar, stepper, content-width cap, and media sizing live in `assets/styles.css:16`, `assets/styles.css:26`, `assets/styles.css:28`, and `assets/styles.css:68`.
- The existing `step` signal already provides the in-app state needed for a separate danger page, while cleanup itself remains native and password-protected (`src/app.rs:921`, `src-tauri/src/library.rs:211`).
- `tauri-plugin-opener` is installed and initialized (`src-tauri/Cargo.toml:22`, `src-tauri/src/lib.rs:205`); an application-owned command can open only the active library folder without accepting a renderer-supplied path.
- Original media must never be moved or deleted without explicit authorization (`context/foundation/prd.md:97`); these UX changes must preserve the existing native cleanup guardrails.

## What We're NOT Doing

- Changing library encryption, unlocking, password recovery, or the native cleanup semantics.
- Creating arbitrary frontend filesystem access or opening a path supplied by the webview.
- Adding routing, external navigation frameworks, or new persistence.
- Changing media discovery, import/skip decisions, tagging, preview asset scope, or source-media safety behavior.
- Adding duplicate, similarity, history, search, or library-management features outside this UX refinement.

## Implementation Approach

Use the existing Dioxus `step` signal to distinguish pre-library onboarding from the selected-library workspace. Keep the initial folder/loading/stale experience branded, then apply a single-column layout modifier and render no sidebar or progress controls for setup, unlock, recovery, home, review, and the new danger page. Retain native ownership of OS-folder launching: a no-argument Tauri command resolves the currently unlocked library location and asks the opener plugin to open that directory.

## Critical Implementation Details

The OS-folder command must not accept a path argument from the frontend. It must derive the active library path from trusted native state, reject a locked or unavailable library with the established structured error pattern, and open the directory contents rather than merely revealing its parent.

The review view must use a viewport-aware flex/grid boundary that reserves space for context, tags, date, and decision buttons before sizing the media area. Preserve `object-fit: contain` for both image and video previews, and include a compact-screen fallback that keeps controls reachable.

## Phase 1: Establish the selected-library workspace

### Overview

Deliver the first visible UX change: selecting a library path transitions the app into a clean, centered full-window workspace without the blue panel, numerical stepper, or step-number wording.

### Changes Required:

#### 1. View-state-driven application shell

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Keep branded onboarding only until a library path has been selected, then give all selected-library views a focused workspace.

**Contract**: Derive an explicit layout mode from `step`. Folder selection, initial loading, and stale-library recovery keep the existing branded shell; setup, unlock, existing-library, recovery, home, review, and danger views render in the centered full-window layout. Remove the sidebar and progress-row markup from the selected-library experience rather than merely hiding it visually.

#### 2. Remove multi-step presentation

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Eliminate language and decoration that imply an obsolete two-step setup process.

**Contract**: Remove the `1 → 2` progress component, its state-derived classes, its CSS, and the numbered "Step 1 of 2" / "Step 2 of 2" labels. Preserve useful screen titles and accessible context without introducing replacement step numbering.

### Success Criteria:

#### Automated Verification:

- `cargo check --workspace` succeeds after the Dioxus layout-state changes.
- A source search confirms that the removed progress-row classes and numbered step labels are no longer rendered or styled.

#### Manual Verification:

- Selecting a new or existing library immediately shows the centered single-column workspace through setup, unlock, recovery, home, review, and danger-page transitions.
- Initial folder selection and stale-library recovery retain the branded onboarding context.
- No blue sidebar, numbered progress row, or numbered step wording appears after a library path has been selected.

**Implementation Note**: After automated verification passes, pause for human confirmation of the post-selection workspace across setup and unlock before continuing.

---

## Phase 2: Simplify library-home actions and isolate cleanup

### Overview

Make the protected-library path the home-action anchor: users can open its contents in the OS file manager, access cleanup from a separate page, and no longer see manual lock controls.

### Changes Required:

#### 1. Native active-library folder opener

**Files**: `src-tauri/src/lib.rs`, `src-tauri/src/library.rs`, `src/app.rs`

**Intent**: Open the current managed library directory in Finder on macOS and Explorer on Windows without giving the frontend a general path-launching capability.

**Contract**: Register a no-argument `open_library_folder` command that obtains the active unlocked library path from native state, validates it remains available, and calls the installed opener plugin to open the directory. Return a structured, user-displayable error for locked, stale, or OS-launch failures. The Dioxus handler invokes this command only from the protected-library summary and provides busy/error feedback.

#### 2. Home action hierarchy and manual-lock removal

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Put routine library navigation and the destructive entry point where users expect them, while removing manual locking from every UI state.

**Contract**: Place an accessible platform-neutral "Open folder" button beside the protected-library path. Move "Open danger zone" immediately beneath that summary, ahead of import-source actions. Remove the home and completion lock controls plus their unused frontend handler; retain the native lock command as an internal session/security primitive unless native ownership proves it unused.

#### 3. Dedicated danger-zone page

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Give destructive cleanup its own focused screen instead of expanding a sensitive password form inline on home.

**Contract**: Add a `danger` view to the existing `step` state machine, entered from the home action and containing the current password confirmation and irreversible-action explanation. Include a non-destructive Back action to home. Reuse the existing native cleanup command and its password requirement; on successful cleanup reset the in-memory source/review UI state and return directly to normal home. Remove the obsolete inline-confirmation state and handlers.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including existing native cleanup and lock/session tests.
- `cargo check --workspace` succeeds with the new command registration and frontend invocation contract.
- Focused native tests cover active-path resolution and rejection when no unlocked/available library can be opened, without invoking the platform file manager in unit tests.

#### Manual Verification:

- From unlocked library home, Open folder opens the active library's contents in Finder/Explorer; a failure is clear and does not open another path.
- The danger-zone entry appears directly below the protected path, opens a standalone page, and Back returns home without cleanup.
- Successful cleanup returns home with the existing safe cleanup behavior intact; no Lock library control appears on home or completion.

**Implementation Note**: After automated verification passes, pause for human confirmation of the OS-folder action and destructive-flow navigation before continuing.

---

## Phase 3: Keep the complete review card in the viewport

### Overview

Deliver a review layout that fits image/video preview, item context, tags, date, and explicit decisions within the available desktop viewport without page scrolling.

### Changes Required:

#### 1. Viewport-bounded review card and media area

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Prevent large media from pushing the decision controls off-screen while preserving clear, aspect-correct previews.

**Contract**: Mark the review view/card with a dedicated layout class and size it relative to the dynamic viewport. Reserve vertical space for the context and controls, make the image/video region consume only the remaining safe area, and keep `max-width: 100%` with `object-fit: contain` for both media types. Provide responsive rules for the configured minimum window size and narrower layouts without hiding any decision control.

#### 2. Layout regression checks

**Files**: `assets/styles.css`, optionally `src/app.rs`

**Intent**: Make the no-scroll review requirement explicit and guard against reintroducing an unbounded media element.

**Contract**: Add the smallest appropriate source-level coverage/check for the review layout contract and retain existing media-preview fallbacks. Manual verification must cover a tall image, a wide image, and a video at the minimum supported window dimensions; it must confirm the entire card and both decisions are visible without document scrolling.

### Success Criteria:

#### Automated Verification:

- `cargo check --workspace` succeeds after review markup and style changes.
- A targeted source-level check verifies viewport-aware review-card/media constraints and retained `object-fit: contain` behavior.

#### Manual Verification:

- At the configured minimum window size, a tall photo, wide photo, and video each show context, tags, editable date, Skip, and Import without page scrolling.
- Resizing the window preserves aspect ratio, never clips controls, and retains the unavailable-preview fallback.
- Import and Skip still require deliberate user action and preserve the source-media safety behavior.

**Implementation Note**: After automated verification passes, pause for human confirmation using representative images and videos before considering the plan complete.

## Testing Strategy

### Unit Tests:

- Native active-library path resolution rejects locked and unavailable libraries before any OS-launch call.
- Existing cleanup tests continue to prove password validation, Trash behavior, and post-success state reset semantics.

### Integration Tests:

- Tauri command registration and the Dioxus invocation use the same no-argument `open_library_folder` contract.
- `cargo test --workspace` protects existing review/import and cleanup behavior from UX-only regressions.

### Manual Testing Steps:

1. Choose a new library and open an existing library; verify the selected-library workspace appears immediately without sidebar or stepper.
2. Unlock the library, use Open folder, and verify Finder/Explorer opens the managed library contents—not a user-provided or unrelated directory.
3. Open and leave the danger page without action; then complete a password-confirmed cleanup and verify return to normal home.
4. Complete a review with tall/wide images and a video at the app's minimum window size; verify the full card and both decisions remain visible with no page scroll.
5. Verify no manual Lock library control appears on home or completion, while normal app restart/unlock behavior remains intact.

## Performance Considerations

These changes introduce no new media reads, copies, or catalogue queries beyond resolving the active library for an explicit OS-folder action. Viewport sizing must use CSS layout constraints rather than measuring or loading the preview in JavaScript.

## Migration Notes

No catalogue or preference migration is required. The danger-page route is transient UI state; it must not alter the native cleanup contract or persisted library/import-source data unless the user submits the existing confirmed cleanup action.

## References

- UX requirements: `context/changes/review-and-decide-media-ux-changes/requirments.md`
- Product safety guardrail: `context/foundation/prd.md:97`
- Existing review implementation: `context/changes/review-and-decide-media/plan.md:1`
- Application shell and home/review states: `src/app.rs:779`, `src/app.rs:887`, `src/app.rs:941`
- Layout and media-preview styles: `assets/styles.css:16`, `assets/styles.css:68`
- Opener plugin setup: `src-tauri/Cargo.toml:22`, `src-tauri/src/lib.rs:205`
- Existing cleanup safety: `src-tauri/src/library.rs:211`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles.

### Phase 1: Establish the selected-library workspace

#### Automated

- [ ] 1.1 Implement the view-state-driven selected-library workspace and remove multi-step presentation
- [ ] 1.2 Verify workspace compilation and absence of removed stepper artifacts

#### Manual

- [ ] 1.3 Confirm selected-library workspace transitions and retained onboarding states

### Phase 2: Simplify library-home actions and isolate cleanup

#### Automated

- [ ] 2.1 Add and test the native active-library folder opener
- [ ] 2.2 Implement home action hierarchy, no-manual-lock UI, and dedicated danger page
- [ ] 2.3 Verify workspace tests and command-contract compilation

#### Manual

- [ ] 2.4 Confirm Finder/Explorer folder opening and standalone danger-zone navigation

### Phase 3: Keep the complete review card in the viewport

#### Automated

- [ ] 3.1 Implement viewport-bounded review-card and media-preview layout
- [ ] 3.2 Verify compilation and the review layout contract

#### Manual

- [ ] 3.3 Confirm no-scroll review cards for tall/wide images and video at minimum window size

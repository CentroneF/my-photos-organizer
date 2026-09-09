# Single media view — Plan Brief

> Full plan: `context/changes/single-media-view/plan.md`

## What & Why

This change adds a full-window preview to managed-library search, letting users inspect, navigate, tag, reveal, rotate, and safely delete individual managed media. It turns the thumbnail grid into a practical review surface while retaining the app's promise that original source media is never changed or deleted.

## Starting Point

Search already returns ordered active imported media, safe asset URLs, and basic tags, but its cards have no click behavior and display extra details. The Dioxus app already has accessible dialog, video, metadata, and tag-chip patterns; the native layer has no candidate-scoped per-media mutation APIs.

## Desired End State

Users click a compact thumbnail to open an accessible preview with image viewing controls or video playback, full metadata, a collapsible tag editor, and predictable filtered-list navigation. Managed-copy rotation and Trash deletion require confirmation and are validated again by Rust; unsupported media remains inspectable without risky actions.

## Key Decisions Made

| Decision | Choice | Why |
| --- | --- | --- |
| Deletion | Move managed copy to OS Trash, then hide it from search | Recoverable deletion protects user media and leaves catalogue history intact. |
| Rotation | Confirm overwrite of managed copy; no unsupported controls | Both UI and native boundary prevent accidental or unsafe writes. |
| Navigation | Stop at boundaries; after final-item deletion open previous | Preserves the selected search context without wrap-around. |
| Tags | Persist each add/remove immediately | Removes unsaved-state ambiguity and keeps the grid in sync. |
| Metadata | Full existing technical metadata, tags, and trusted path actions | Reuses review capabilities and meets the information-panel requirement. |
| File manager | Open containing folder with Finder/Explorer label | Uses safe cross-platform opener behavior without exposing arbitrary paths. |
| Failure UX | Keep preview open with recovery actions | Users retain navigation and metadata when preview media fails. |
| Verification | Rust safety tests plus desktop manual testing | Native mutations and browser/OS behavior both need coverage. |

## Scope

**In scope:** full-window image/video preview; position and keyboard navigation; viewing controls; metadata/sidebar; tags; copy/reveal; safe rotate; safe Trash deletion; loading/error states.

**Out of scope:** source-media mutations, exact Finder/Explorer file selection, unsupported-codec rotation, deleted-history UI, export/share, ratings, and a new browser test framework.

## Architecture / Approach

`App` retains the current filtered search vector and selected index. Candidate-ID-only Tauri commands re-resolve the active imported managed item and validate its canonical path before supplying details, tags, clipboard/folder actions, rotation, or Trash deletion. The UI uses the existing accessible overlay pattern and refreshes selected state after mutations.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Open and navigate | Accessible preview, media display, metadata, controls, navigation, error recovery | Maintaining focus and result-list context. |
| 2. Manage metadata | Immediate tags, autocomplete, copy confirmation, Finder/Explorer reveal | Avoiding stale filtered results after tag edits. |
| 3. Safely change media | Confirmed rotate and Trash deletion with result reconciliation | Filesystem failure and protection of managed/original boundaries. |

**Prerequisites:** An unlocked protected library containing managed image and video copies; macOS or Windows desktop environment for manual checks.
**Estimated effort:** ~3 focused implementation sessions across 3 vertical phases.

## Open Risks & Assumptions

- Existing image codecs do not safely support every catalogued image format; unsupported formats expose no rotation controls.
- Image re-encoding may not preserve all EXIF data, which is disclosed by overwrite confirmation.
- If a Trash operation succeeds but catalogue update fails, the stale item remains visible only until its next safe resolution reports it missing.

## Success Criteria (Summary)

- Users can inspect and navigate filtered images/videos in a full-window preview with clear recovery states.
- Tag and metadata actions are saved safely and reflect in search state.
- Rotate/delete change only validated managed copies, require confirmation, and never affect source originals.

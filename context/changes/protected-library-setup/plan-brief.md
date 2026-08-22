# Protected library setup — Plan Brief

> Full plan: `context/changes/protected-library-setup/plan.md`

## What & Why

Photo Handler needs a trusted first-run experience before it can catalogue media. This plan lets one person choose a fixed folder, create a protected local library state there, and reopen it later without modifying original media.

## Starting Point

The project is a Tauri/Dioxus starter with one greeting command and no persistence, folder selection, password flow, or catalogue. The PRD requires a fixed user-selected library and local-only password protection.

## Desired End State

An empty chosen folder receives only a dedicated `.photo-handler/` state directory with an encrypted SQLite catalogue. On later launches the app recognizes, validates, and unlocks that library; a local recovery-answer flow can reset the password without cloud or email.

## Key Decisions Made

| Decision | Choice | Why | Source |
| --- | --- | --- | --- |
| Primary store | SQLCipher-backed SQLite via `rusqlite` | Supports portable, transactional state and future relational catalogue/search needs. | Plan |
| Library location | Empty user-selected folder with `.photo-handler/` state | Keeps application state separate and avoids touching arbitrary user content. | Plan |
| Access protection | Password plus custom local recovery question | Meets the requested local-only recovery approach, while documenting its lower assurance. | Plan |
| Credential lifecycle | Random DB key wrapped by password and recovery answer | Lets recovery reset the password without recreating the catalogue. | Plan |
| Reopen behavior | Remembered path plus “Open existing library” fallback | Supports normal restart and moved-library recovery without disk scanning. | Plan |

## Scope

**In scope:** native folder selection/validation, encrypted catalogue bootstrap, password/recovery handling, remembered-path reopening, recovery reset, tests, and setup/unlock UI.

**Out of scope:** media import or encryption, scanning, search, cloud/email recovery, multi-user access, and automatic conversion of arbitrary existing folders.

## Architecture / Approach

Dioxus renders setup and unlock states and calls native Tauri commands. Native Rust validates filesystem state, manages SQLCipher and Argon2id key wrapping, applies versioned migrations, and persists only the selected-library pointer locally. The library folder contains its own marker and encrypted catalogue, so it can be reopened safely after relocation.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Create protected library | First-run folder selection, protected catalogue creation, and a visible success state | Never mutate arbitrary user folders or media. |
| 2. Reopen, unlock, recover | Restart unlock, validated reopen, and local password reset | Recovery is intentionally weaker than a recovery key. |

**Prerequisites:** macOS/Windows filesystem access and native dependency builds for SQLCipher/Argon2id.
**Estimated effort:** ~3–5 focused sessions across 2 phases.

## Open Risks & Assumptions

- Security questions are accepted by product choice but are weaker than recovery keys and must be described honestly in the UI.
- A forgotten password and recovery answer permanently prevent access; no cloud recovery is planned.
- The chosen folder must be empty at setup; only a recognised compatible marker may be reopened.

## Success Criteria (Summary)

- A user can create a protected fixed library from the desktop UI without any media operation.
- The same library reopens after restart with a password, or locally resets after its recovery answer.
- Invalid folders and incorrect credentials fail without altering library or user-folder contents.

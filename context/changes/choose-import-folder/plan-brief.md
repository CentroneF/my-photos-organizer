# Choose import folder — Plan Brief

> Full plan: `context/changes/choose-import-folder/plan.md`

## What & Why

Photo Handler needs a safe way to identify the folder from which a later review/import flow will work. This change lets a user choose and remember one separate source folder after their protected library is ready, without touching that folder's media or claiming that an import has begun.

## Starting Point

The completed protected-library flow can create or unlock the fixed managed library, but its final screen has no ongoing library action. Its existing picker deliberately validates only empty or recognized library folders, so a normal non-empty media folder cannot be selected as an import source today.

## Desired End State

After setup, unlock, or recovery, the user lands on a library home where they can choose an import folder, see its path, and change it. The latest choice survives restart; if it becomes unavailable, the app preserves and labels the remembered path while offering a replacement action. The protected library itself cannot be selected as the source, and source contents remain untouched.

## Key Decisions Made

| Decision | Choice | Why | Source |
| --- | --- | --- | --- |
| Entry point | Library home after the library is ready | Separates one-time setup confirmation from ongoing import preparation. | Plan |
| Source eligibility | Any directory from the picker; no media scan | Meets FR-002 without prematurely building review/discovery behavior. | Plan |
| Persistence | Latest selected source in app-local settings | Retains the user's choice across restarts without defining an import-session catalogue record. | Plan |
| Unavailable source | Retain and label it stale | Preserves useful context while avoiding a false ready state. | Plan |
| Library/source separation | Reject the managed-library root | Prevents future import logic from treating protected application state as source media. | Plan |

## Scope

**In scope:** a library-home UI, native folder selection, one persisted source-path pointer, available/missing/stale source state, same-as-library rejection, tests, and manual desktop verification.

**Out of scope:** media scanning or validation, import queues, review, tagging, copying, moving, deleting, hashing, duplicates, catalogue migration, and cloud behavior.

## Architecture / Approach

The Dioxus root component invokes the already-enabled native directory picker, then calls a new native import-source command. Native Rust performs metadata-only validation, rejects a canonical path equal to the remembered protected-library root, and persists a separate JSON pointer with the repository's temporary-file-and-rename pattern. It returns a small state object so the frontend can render ready, missing, or stale selection states.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Choose and remember an import source | Complete picker, validation, persistence, and library-home UI | Maintaining the hard separation between an untouched source and the managed library. |

**Prerequisites:** the protected-library setup/unlock flow is available; the existing dialog plugin and capability remain enabled.
**Estimated effort:** ~1–2 focused sessions in one vertical phase.

## Open Risks & Assumptions

- A selected source can become unavailable between application launches; it is shown as stale rather than cleared automatically.
- This slice validates only directory availability and library-root overlap. Media suitability belongs to review/import work.
- The chosen path is app-local convenience state, not an import record; replacing it does not affect source contents or the protected catalogue.

## Success Criteria (Summary)

- A user can select and later change a source folder from the desktop library home.
- The selected source remains available after restart when it still exists, otherwise it is clearly marked unavailable.
- The managed library cannot be selected as source, and verification confirms no source-folder mutation or media processing occurs.

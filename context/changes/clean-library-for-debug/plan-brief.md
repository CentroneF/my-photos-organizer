# Clean library for debug — Plan Brief

> Full plan: `context/changes/clean-library-for-debug/plan.md`

## What & Why

Photo Handler will gain a deliberate debug action that returns a managed library to its just-set-up state. It moves managed copied-media date folders to the operating system Trash, clears picture/review metadata and the remembered import source, and keeps the protected library setup intact.

## Starting Point

The protected library stores its marker and encrypted catalogue in `.photo-handler/`; imported copies live directly in root date folders and their review metadata is held in the SQLCipher catalogue. The app has no cleanup path today, and an interrupted import can leave an unrecorded copy in the same date layout.

## Desired End State

An unlocked user can enter the current library password in a danger-zone confirmation flow and clean all managed date-folder copies. Originals, unrelated root content, the protected catalogue identity, credentials, and the selected-library pointer remain untouched; the app remains unlocked and ready for a new import source.

If any folder cannot be moved to Trash, cleanup reports a retryable incomplete state and preserves all metadata until a later successful retry.

## Key Decisions Made

| Decision | Choice | Why |
| --- | --- | --- |
| Authorization | Re-enter current library password | Makes destructive intent explicit and verifies the active user. |
| Copy disposal | Native OS Trash/Recycle Bin | Provides the OS-supported recovery path instead of permanent deletion. |
| Owned media scope | Top-level `YYYY/YYYY-MM-DD` folders only | Removes managed copies and import orphans while preserving unrelated root content. |
| Metadata timing | Clear only after every target is trashed | Retains enough state for safe retry after a partial failure. |
| Import source | Forget it on success | Returns the debug library to its post-setup state. |
| Session state | Stay unlocked | Lets the user immediately select a fresh source and continue debugging. |

## Scope

**In scope:** password-confirmed clean action; native Trash integration; root date-folder classification; review/catalogue cleanup; import-source reset; retryable partial failures; desktop UI; safety tests.

**Out of scope:** source-original changes; arbitrary root deletion; emptying/restoring OS Trash; reset of credentials or library identity; search/duplicate work.

## Architecture / Approach

The Dioxus home screen invokes one native cleanup command. Rust validates the unlocked session and fresh password, moves only eligible top-level year directories to the OS Trash, and clears mutable catalogue data plus the source pointer in a transaction only after every target succeeds. Any Trash failure keeps all metadata for a safe retry.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Password-confirmed library clean | End-to-end danger-zone clean with setup/source preservation | Selecting only managed media, never user/root/source content. |
| 2. Partial-cleanup recovery | Retryable failure state and deterministic safety regression coverage | Avoiding metadata loss after only some folders reached Trash. |

**Prerequisites:** Existing protected-library and review/import flows; a disposable native library for manual tests.
**Estimated effort:** ~2–3 focused sessions across 2 vertical phases.

## Open Risks & Assumptions

- OS Trash availability and permissions can fail, so no partial cleanup may claim success.
- The strict date-folder layout is the ownership boundary; nonconforming copied files are intentionally preserved rather than guessed at.
- OS Trash recovery remains subject to the user not later emptying the Trash/Recycle Bin.

## Success Criteria (Summary)

- A password-confirmed clean sends all eligible managed-copy folders, including unrecorded copies inside them, to the native Trash without touching originals or unrelated content.
- A successful clean removes all review/history metadata and the remembered source while the protected library remains unlockable.
- A partial Trash failure is visible and retryable, preserving metadata until cleanup can finish.

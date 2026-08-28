# Frame Brief: Resume newly added media

> Framing step before implementation. This document separates the observed
> behavior from the assumptions behind it.

## Reported Observation

“i added a file to an already reviewed folder, and when i click resume, it
shows the review complete screen, instead of proposing me to import the new
file”

## Initial Framing (preserved)

- **User's stated cause or approach**: A completed source appears to remain complete even after its contents change.
- **User's proposed direction**: Resume should scan a completed source for newly added supported files and present only those files.
- **Pre-dispatch narrowing**: The desired behavior is explicit resume-time discovery of new files, not background watching or re-review of decided files.

## Dimension Map

The observation could originate at any of these dimensions:

1. **Home action selection** — the UI may invoke a completion-only path instead of a resume operation.
2. **Session lookup** — a completed session may be returned before source discovery. ← initial framing
3. **Candidate persistence** — the schema may be unable to retain prior decisions while adding new candidates.
4. **Completion rendering** — the next-item command may select a completed session despite new pending work.

## Hypothesis Investigation

| Hypothesis | Evidence | Verdict |
| --- | --- | --- |
| Home action selection | Both Start and Resume invoke `start_review`, then `next_review_item` (`src/app.rs:406`, `src/app.rs:827`). | NONE |
| Completed session short-circuits discovery | `start_review` returns a matching complete session before calling `discover` (`src-tauri/src/review.rs:103`, `src-tauri/src/review.rs:119`). | STRONG |
| Decisions cannot be preserved during reconciliation | Candidates are unique per `(session_id, relative_path)` and decisions are stored per candidate (`src-tauri/src/library.rs:618`). | NONE |
| Completion rendering is independently wrong | `next_review_item` returns completion for the latest completed session (`src-tauri/src/review.rs:153`). This follows from the short-circuit. | WEAK |

## Narrowing Signals

- The user explicitly confirmed that resume should scan a completed source for newly added supported files.
- Discovery already performs a read-only recursive scan; it does not modify originals (`src-tauri/src/review.rs:416`).

## Cross-System Convention

An explicit rescan is the expected local-first import behavior: preserve recorded decisions and identify only new source-relative paths. This remains within the existing no-background-watching scope.

## Reframed Problem Statement

> **The actual problem to address is**: explicit resume does not reconcile a completed session with newly discovered source-relative candidates.

The fix must retain all existing decisions and tags, add only new candidate paths in one transaction, and reactivate the session only when additions exist. It must not treat changed file contents as new media, modify the source, or create a second session for the same source.

## Confidence

- **HIGH** — code-path evidence directly accounts for the behavior, the schema preserves the required identity boundary, and the user confirmed the intended behavior.

## What Changes for Implementation

Extend explicit resume-time discovery for completed sessions, with regression coverage for adding a file after completion and preserving prior decisions. The existing vertical review flow remains the user-visible verification path.

## References

- Source files: `src-tauri/src/review.rs:89`, `src-tauri/src/review.rs:153`, `src-tauri/src/library.rs:618`, `src/app.rs:406`
- Investigation tasks: `/root/session_model`, `/root/decision_safety`

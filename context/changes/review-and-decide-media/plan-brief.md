# Review and decide media — Plan Brief

> Full plan: `context/changes/review-and-decide-media/plan.md`

## What & Why

Photo Handler will gain its first complete media-review workflow. A person will review common photos and videos from a selected source, tag each item, adjust its import date, then explicitly import a safe copy or skip it; the original is never modified.

This is the essential path between choosing an import folder and later duplicate context/search. It converts the current remembered-folder convenience state into durable, protected catalogue decisions that the person can resume after restarting.

## Starting Point

The completed app can create/unlock an encrypted managed library and remember one separate source folder, but home currently only offers Choose/Change folder. The SQLCipher catalogue has no media, tags, sessions, or decisions, and unlock immediately clears its database key after validation.

## Desired End State

From library home, a user starts or resumes a recursive review of a selected source. Each supported image/video is presented one at a time with an in-app preview, tags, and editable date. Import copies it to `<managed library>/<year>/<date>/` under a unique filename; Skip persists across restart. Completion reports counts and confirms that originals were not moved or deleted.

## Key Decisions Made

| Decision | Choice | Why | Source |
| --- | --- | --- | --- |
| Import semantics | Copy into managed library | Preserves originals while making the library independently managed. | Plan |
| Source discovery | Recursive through subfolders | Matches the requested import behavior for nested backups. | Plan |
| Video review | Play in app | A video must be reviewable before its explicit decision. | Plan |
| Skip durability | Persist immediately | A completed skip must survive restart instead of creating repeat work. | Plan |
| Destination layout | `/<year>/<date>/` | Keeps imported media date-organized with a clear editable assignment. | Plan |
| Date precedence | Metadata, then created date, then user edit | Uses the best available date while preserving user control. | Plan |
| Filename collisions | Generate a unique filename | Avoids overwriting managed media or requiring an interruption. | Plan |
| Review continuity | Resume or choose another source | Lets users pause work without losing its context. | Plan |
| Duplicate/history scope | Defer suggestions and hashing to S-04 | Preserves the roadmap boundary while retaining minimal durable decisions. | Roadmap / Plan |

## Scope

**In scope:** encrypted catalogue migration; authenticated in-memory session; recursive common-media discovery; source-safe previews; photo/video review UI; tags; editable date; durable skip/import decisions; atomic date-folder copies; resume/completion flow; native and manual verification.

**Out of scope:** deletion/moving of originals, hashes, duplicate or visual-similarity suggestions, library search, custom destination schemes, cloud/shared libraries, and background watching.

## Architecture / Approach

Native Rust owns session keys, SQLCipher access, source traversal, preview URLs, and file copies. Dioxus renders home and review states using serializable command DTOs. A transactional catalogue migration stores sessions/candidates/tags/decisions, while scoped asset access permits only user-selected media previews rather than broad frontend filesystem access.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Start and resume | Protected session, schema, safe recursive queue, home entry | Source/library overlap and preserving source contents. |
| 2. Review and decide | Previews, tags, date override, skip, atomic copy | Never publishing a partial import or altering source media. |
| 3. Complete and harden | Completion/recovery UI and lifecycle coverage | Reliable resume and platform codec fallback. |

**Prerequisites:** the existing protected-library and import-source flows; native media samples for manual desktop verification.
**Estimated effort:** ~4–6 focused sessions across 3 vertical phases.

## Open Risks & Assumptions

- Embedded webview codec support can vary by macOS/Windows installation; an unsupported video must show a clear item-level fallback and remain undecided.
- Metadata date extraction is format-dependent; creation time is the defined fallback, and the person can override either result.
- An imported file that publishes but cannot be recorded requires an explicit recovery state; it must never be overwritten or automatically copied again.

## Success Criteria (Summary)

- A person can start/resume a review, preview common photos/videos, tag an item, import a copied date-organized file, or skip it.
- Imported and skipped decisions survive restart, and users can choose another source without losing unfinished work.
- Verification demonstrates no source media is moved, deleted, renamed, or changed during discovery, decisions, copy failures, or collisions.

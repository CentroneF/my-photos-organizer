# Managed library search — Plan Brief

> Full plan: `context/changes/search-managed-library/plan.md`

## What & Why

After unlocking, Photo Handler will open directly to a full-window search view of imported managed media rather than an import-first home. Users can narrow a visual image/video grid by date range, media type, and suggested tags while keeping all catalogue data and media access local.

## Starting Point

The protected local catalogue already persists imported decisions, normalized tags, managed destinations, media type, and reviewer-selected import dates. It has no library-search API or UI; the current post-unlock view starts by choosing an import folder.

## Desired End State

An unlocked library opens to a responsive imported-media grid that uses the full application window. An empty library provides a clear Import media action; populated libraries support image/video filtering, date-range filtering, safe previews, tag suggestions, and an explicit choice between original media date and selected import date.

## Key Decisions Made

| Decision | Choice | Why | Source |
| --- | --- | --- | --- |
| Default unlocked view | Library Search | Discovery is the primary library experience; import remains an explicit action. | Plan |
| Search population | Imported only | Managed-library results must exclude skipped and pending source candidates. | Plan |
| Filters | Date range, tags, media type | These are the requested structured discovery fields and satisfy FR-009. | Plan |
| Date model | Persist original and selected dates | Search must retain both meanings when a review overrides a discovered date. | Plan |
| Tag UX | Suggest after two characters | Reuses normalized catalogue tags while avoiding an always-visible lookup control. | Plan |
| Preview behavior | Scoped cards with fallbacks | Preserves local-file security without hiding unsupported image/video results. | Plan |

## Scope

**In scope:**

- Default-after-unlock imported-media search grid and an import-oriented empty state.
- Date-range, media-type, and multi-tag AND filtering.
- Safe per-result previews, original-date persistence, and two-character tag suggestions.

**Out of scope:**

- Skipped/pending history, full-text search, thumbnails, maps, similarity, pagination, and source-media mutation.

## Architecture / Approach

A new native search module queries only imported rows through the existing authenticated encrypted-catalogue seam. It validates each stored destination against the active library root before granting a dynamic asset URL; Dioxus consumes the result through registered Tauri commands and renders the default Library Search state in the existing single-component step machine.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Default library search and empty state | Full-window imported grid, selected-date/type filters, safe previews, and Import media empty state | Stored paths must never widen asset access. |
| 2. Original-date search and tag suggestions | Persisted original date, date-mode selection, and suggested multi-tags | Existing records cannot reliably gain original dates. |

**Prerequisites:** The implemented review/import slice and an unlocked protected library.
**Estimated effort:** ~2–3 sessions across 2 vertical phases.

## Open Risks & Assumptions

- HEIC and some video codecs may not render in the desktop webview; their cards must remain usable with metadata fallback.
- Original-date filtering will only cover imports made after the new nullable fields exist; legacy entries remain searchable by selected import date.
- The grid operates on a local catalogue and does not add cloud transfer or broad filesystem permission.

## Success Criteria (Summary)

- Unlocking opens Library Search; an empty library exposes a safe, clear import path.
- Only imported media appears, and date/type/tag filters produce the expected grid.
- Original media remains untouched and every preview URL is restricted to a verified managed destination.

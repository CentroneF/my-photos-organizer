# Filters UX — Plan Brief

> Full plan: `context/changes/filters-ux/plan.md`

## What & Why

This change turns Library Search’s top filter form and four-button header into a clearer media workspace. Filters move into independently expandable sections on the left, active constraints become removable chips above the grid, and routine library actions move into one top-right settings menu.

It also fills functional gaps in the current search contract: Imported and Captured dates become independent ranges, media types become checkboxes, and tags become a frequency-ranked, searchable discovery control.

## Starting Point

Library Search already safely queries active imported copies from the encrypted catalogue and renders their managed previews. Its current Dioxus UI has one date-mode range, one media select, a prefix-only tag-suggestion field, and four always-visible header actions.

## Desired End State

Users can leave several left-side filter sections open, compose date/media/tag criteria, and immediately see a precise grid with every active filter visible above it. They can remove one chip or clear the entire filter set without losing their place.

The action menu retains access to Import media, Library settings, Close library, and Danger zone. Tag discovery starts with ten common imported tags and supports literal substring searching; selected tags combine with AND semantics.

## Key Decisions Made

| Decision | Choice | Why |
| --- | --- | --- |
| Date behavior | Both filled ranges must match | Separate Imported and Captured controls remain literal and predictable. |
| Media checkboxes | Both selected initially; none means no results | Uses standard checkbox semantics without ambiguous empty state. |
| Tag discovery | Top 10 plus literal substring search | Keeps the panel compact while allowing discovery beyond common tags. |
| Tag matching | Require every selected tag | Preserves the established precise AND semantics. |
| Filter feedback | Removable chips plus Clear all | Makes composed constraints visible and quickly reversible. |
| Disclosure | Multiple filter sections stay open | Supports adjusting combined filters efficiently. |
| Library actions | One anchored top-right menu | Declutters the header without removing existing routes. |
| Search timing | Apply immediately | Keeps results and applied-filter summary synchronized. |

## Scope

**In scope:**

- Left-side expandable filter workspace, applied-filter bar, responsive styles, and action menu.
- Independent Imported/Captured date ranges and multi-type media search.
- Top-ten, frequency-ranked, imported-only tag discovery with literal substring search.
- Native and frontend regression coverage plus manual verification per phase.

**Out of scope:**

- Saved filters, full-text filename search, tag editing, pagination, FTS, tag counts, OR tag mode, and preview/security changes.

## Architecture / Approach

The Dioxus `home` view remains the search workspace and serializes state to the existing `search_library` Tauri command. Native code continues to query through the authenticated encrypted-catalogue seam; only its date/media request representation and a new tag-list command change. The managed-preview guard remains untouched.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Workspace shell | Left panel, filter chips, actions menu, label cleanup | Preserve existing routes and responsiveness. |
| 2. Date ranges | Two AND-combined ranges end to end | Legacy captured dates are null. |
| 3. Media checkboxes | Multi-type selection end to end | Empty selection must be unambiguous. |
| 4. Tag discovery | Top-ten tags and substring search end to end | Keep matching literal and imported-only. |

**Prerequisites:** An unlocked protected library with varied imported media; fixture data for automated coverage.
**Estimated effort:** ~4 focused implementation sessions across four manually verifiable phases.

## Open Risks & Assumptions

- `%term%` substring matching can be less index-friendly than prefix search; responses are capped at ten and stay local.
- Legacy imported records without captured dates must remain understandable in the UI.
- The plan assumes the current source/CSS contract-test pattern remains the appropriate frontend coverage baseline.

## Success Criteria (Summary)

- The Library Search workspace is cleaner, responsive, and retains every existing library action.
- Every requested filter accurately and immediately narrows only active imported managed media, with clear reversible state.
- Tag discovery ranks common tags, finds literal substrings safely, and preserves AND matching across selected tags.

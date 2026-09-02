# Review Media Page UX Changes — Plan Brief

> Full plan: `context/changes/review-media-page-ux-changes/plan.md`

## What & Why

The review page will become a full-window workspace for making informed import decisions. It shows the approved Media details fields plus GPS coordinates and persists only GPS coordinates with each imported media item for later search work, alongside quick multi-tag selection and safe comparison decisions.

## Starting Point

Review currently uses a narrow two-column card with a comma-separated tag input and advisory similar-match history. Imports create a managed copy and record a decision, but there is no transactional replacement or tag-transfer capability.

## Desired End State

Every review candidate has a responsive information workspace, removable tags, recent-tag shortcuts, and Media details for type, size, dimensions, created, modified, captured, camera, orientation, and GPS coordinates. A successful import persists GPS coordinates for future search. A reviewer can compare against every similar import, keep both, skip the current item, or substitute a selected old managed copy; replacement preserves a recoverable path on failure and transfers the normalized union of old/current tags.

## Key Decisions Made

| Decision | Choice | Why |
| --- | --- | --- |
| Replacement behavior | Remove the old managed copy only after successful replacement | Delivers true substitution without touching source originals. |
| Tag transfer | Normalized union of old and current tags | Preserves prior organization and current review input. |
| Metadata scope | Media details plus GPS coordinates | Avoids a raw EXIF dump while retaining the details needed during review. |
| Metadata persistence | Persist GPS now; search UI later | Keeps Phase 1 focused while preparing GPS search data. |
| Similar-match access | Every match, newest-first | Meets the requirement to compare each similar imported picture. |
| Comparison UX | Per-match side-by-side modal | Keeps each decision focused and visually clear. |
| Dialog dismissal | Escape or explicit Close/Cancel only | Protects in-progress comparison while preserving keyboard access. |
| Review tags | Space-committed removable chips | Optimizes quick multi-tag entry; multi-word review tags are not supported. |
| Recent tags | Five most recent imported tags | Aligns shortcuts with the managed-library boundary. |

## Scope

**In scope:**

- Full-window review layout, Media details/GPS display, GPS persistence, tag chips, and five recent imported tag shortcuts.
- All similar imported matches and an accessible comparison dialog.
- Keep Both, Skip, and a recoverable Substitute workflow with merged tags.
- Catalogue migration, native commands, automated coverage, and manual verification.

**Out of scope:**

- Mutating source originals, metadata-search controls, exact-match comparison, thumbnail generation, or a standalone duplicate-management view.

## Architecture / Approach

The Dioxus review component gains Media details/GPS, tag, and dialog state while native review DTOs extract the approved fields. Imported decisions persist an optional decimal GPS pair in the encrypted catalogue without adding metadata-search filters yet. A recent-tags query uses the same catalogue. Substitute is a dedicated native operation: publish the incoming managed copy, protect the old copy with a recovery link, update catalogue state transactionally, remove the old managed copy, then commit or compensate on failure.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Full-Window Review Information and Tags | Full-window Media details/GPS workspace, GPS persistence, and multi-tag review input | Unsupported metadata must never block decisions; migration must preserve existing imports. |
| 2. Compare Every Similar Imported Item | All-match list and accessible comparison dialog | Dialog focus/error state and unbounded list scrolling. |
| 3. Safe Substitute and Tag Transfer | Recoverable replacement and merged tags | Filesystem/database compensation must prevent managed-copy loss. |

**Prerequisites:** Existing `review-media-page-ux-changes` branch and encrypted-catalogue test setup.
**Estimated effort:** ~3 focused implementation sessions across 3 vertical phases.

## Open Risks & Assumptions

- The approved space delimiter intentionally prevents multi-word tags in this review UI; existing multi-word tags remain readable but cannot be newly entered there.
- Filesystem and SQLite commits cannot be one atomic operation, so recovery-link creation and compensation coverage are essential.
- Some formats, especially unsupported images/videos, will have partial metadata; the UI must label unavailable standard values rather than fail review.
- Metadata search is intentionally deferred even though GPS coordinates are persisted.

## Success Criteria (Summary)

- Review is fully usable across the window, including the approved Media details, GPS, and rapid tag selection.
- Every similar imported item can be compared and decided without unsafe dialog behavior.
- Substitute transfers tags, hides the superseded import, preserves source originals, and leaves actionable recovery state on failure.

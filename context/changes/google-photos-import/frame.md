# Frame Brief: Google Takeout duplicate cleanup review

> Framing step before /10x-plan. This document captures what is *actually*
> at issue, separated from what was initially assumed.

## Reported Observation

The user wants to find both exact and visually similar duplicates in Google
Photos, retain important GPS metadata, and clean unwanted copies from Google
Photos.

## Initial Framing (preserved)

- **User's stated cause or approach**: Connect to the Google Photos API, download pictures and metadata, then use it to delete pictures not worth keeping.
- **User's proposed direction**: Analyze a Google Takeout export for duplicates and create a cleanup review list.
- **Pre-dispatch narrowing**: Both exact byte-identical duplicates and visually similar pictures matter.

## Dimension Map

The observation could originate at any of these dimensions:

1. **Exact cross-export identity** — repeated archives may have different folder names, timestamps, or filenames even when their media bytes are identical.
2. **Visual association** — edited/re-encoded images may differ by bytes while depicting the same subject; not every image/video format is currently comparable.
3. **Takeout metadata ingestion** — Google sidecar JSON can contain useful provenance/GPS but is currently not associated with a media candidate.
4. **Google Photos deletion authority** — a local duplicate result is not equivalent to API authority to remove the corresponding existing cloud item. ← initial framing

## Hypothesis Investigation

| Hypothesis | Evidence | Verdict |
| --- | --- | --- |
| Byte-identical items from separate exports are recognized | `next_review_item` streams BLAKE3 and performs a global digest lookup independent of source path/name (`src-tauri/src/review.rs:306-341`, `723-760`, `891-946`; `library.rs:807`). | **STRONG** |
| Visual matches need advisory, format-aware treatment | Current 64-bit dHash supports JPEG/PNG/WebP/GIF within decoded-pixel limits; it excludes exact matches and does not support HEIC or videos (`review.rs:949-1065`, `1602-1611`). | **STRONG** |
| Existing product loses Takeout-only metadata | Discovery intentionally accepts media extensions only, so JSON sidecars are ignored; metadata is extracted from local media EXIF only (`review.rs:778-868`, `1157-1216`). | **STRONG** |
| Google API can delete undesired existing library items | Google limits Library API access/manage operations to app-created media and exposes no existing-user-media deletion route; Picker is read-only selection. [Authorization](https://developers.google.com/photos/overview/authorization), [Library REST reference](https://developers.google.com/photos/library/reference/rest). | **NONE** |

## Narrowing Signals

- The user explicitly needs both certainty for exact duplicates and a human review for visually similar pictures.
- The product guardrail requires source preservation and user-controlled decisions; the existing duplicate UI is advisory rather than destructive.
- Google removes location metadata from Picker downloads, while Takeout is a local archive route that can retain accompanying sidecar information.

## Cross-System Convention

Photo Handler already treats a content fingerprint as exact history and perceptual similarity as bounded context before a human Import/Skip decision. The PRD requires original media to remain untouched unless explicitly authorized. A Google Takeout workflow should extend that same evidence-first, local-only review model; it cannot promise remote mutation that Google does not authorize.

## Reframed Problem Statement

> **The actual problem to plan around is**: let a user locally analyze a Google Takeout export, reconcile its media and sidecar metadata, and review trustworthy exact and possible visual duplicates in an explicit cleanup queue—without claiming that the app can delete the matching items from an existing Google Photos library.

The existing BLAKE3/dHash foundation already answers part of the need, but it must ingest Takeout metadata and communicate confidence/format limits clearly. The cleanup result must be a user-reviewed, actionable record for manual deletion in Google Photos, not a remote synchronization or bulk-delete feature.

## Confidence

- **HIGH** — the local duplicate behavior and metadata gap are verified in the current code, and Google's public API restriction is explicit.

## What Changes for /10x-plan

Plan a vertical Google Takeout analysis/review slice that reuses exact and visual detection, associates supported sidecar metadata such as GPS, and produces an explainable manual Google Photos cleanup queue. Do not include Google OAuth, Google Photos Picker, remote sync, or deletion.

## References

- Exact/visual duplicate behavior: `src-tauri/src/review.rs:306-341`, `723-760`, `891-1065`, `1157-1216`, `1602-1611`
- Catalogue fingerprint fields: `src-tauri/src/library.rs:807`
- Product guardrail: `context/foundation/prd.md`
- Related research: `context/changes/google-photos-import/research.md`
- Google API restriction: [Authorization scopes](https://developers.google.com/photos/overview/authorization), [Library REST reference](https://developers.google.com/photos/library/reference/rest)

# Similar-picture context during import review — Plan Brief

> Full plan: `context/changes/show-duplicate-and-history-context/plan.md`
> Frame brief: `context/changes/show-duplicate-and-history-context/frame.md`
> Research: `context/changes/show-duplicate-and-history-context/research.md`

## What & Why

> **The actual problem to plan around is**: During review of a new image, Photo Handler lacks a trustworthy, bounded way to show visually similar images that are already safely imported in the managed library, while preserving explicit user decisions and separate skip-history records.

This plan adds advisory exact and similar-picture context to the existing review card. It never automates Import or Skip, and it never changes original source media.

## Starting Point

The encrypted catalogue already stores review candidates, decisions, tags, and imported managed destinations, and `next_review_item` already powers the first and subsequent review cards. It has no content or visual fingerprints; resume also currently treats a source-relative path as the whole identity of a candidate.

## Desired End State

Reviewing a supported still image shows at most three **Possible similar pictures** from the managed imported library, with safe thumbnails and clear qualitative labels. Exact equal-byte history additionally identifies prior imports or skips; skipped history stays metadata-only. Unsupported or unsafe visual comparisons state that they are unavailable rather than implying no match exists.

## Key Decisions Made

| Decision | Choice | Why | Source |
| --- | --- | --- | --- |
| Visual scope | JPEG, PNG, WebP, GIF first | Delivers the primary picture-review value without a video decoder/distribution project. | Plan |
| Exact signal | BLAKE3-256 for all supported media | Gives reliable imported/skip history even when visual decode is unavailable. | Research |
| Candidate replacement | Append a new revision | A changed file must not inherit an older file’s decision or fingerprint. | Plan |
| Similarity UI | Up to three compact thumbnails | Supports visual judgment without crowding explicit actions. | Plan |
| Similarity contract | Versioned 64-bit dHash, fixture-calibrated threshold 10 | Keeps the initial claim testable and lets future recalibration be explicit. | Plan |
| Failures | Inline unavailable status, decisions enabled | Maintains honest advisory context and the established explicit-decision workflow. | Plan |
| Preview boundary | Imported managed copies only | Skipped source files are history, not new webview filesystem scope. | Frame / Research |

## Scope

**In scope:**

- Encrypted exact and perceptual fingerprints, format-6 migration, and revision-aware resumption.
- Imported/skipped exact-history context and imported-only similar-picture context in review.
- Safe managed-copy thumbnails, resource/decode unavailable states, fixtures, and desktop verification.

**Out of scope:**

- Video similarity, HEIC visual decoding, automatic decisions, source mutation, broad filesystem access, and search-scope changes.

## Architecture / Approach

`next_review_item` fingerprints the active source through the existing authenticated SQLCipher boundary, persists only stable results, then queries bounded context. Exact matches span decisions; perceptual matches span imported managed images only. A shared managed-preview guard grants asset URLs only after containment, readability, and non-symlink checks; Dioxus renders the serialized advisory result alongside unchanged decision controls.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Exact history | Format-6 schema, revision-safe candidates, exact imported/skip context | Never reuse a decision for replaced bytes. |
| 2. Similar pictures | Bounded still-image dHash, fixture calibration, thumbnail context | Avoid false certainty and review-card crowding. |
| 3. Hardening | Recovery, migration/restart, and desktop end-to-end proof | No false absence claim or preview-scope leak. |

**Prerequisites:** Implemented review/import flow and an unlocked protected library.
**Estimated effort:** ~4–6 sessions across 3 vertical phases.

## Open Risks & Assumptions

- The threshold is deliberately calibrated only against committed non-personal fixtures; real-world feedback may justify a future versioned recalibration.
- HEIC, video, corrupt, and oversized files retain exact history but not visual comparison in this change.
- Full visual video similarity remains a later framed product/package decision; it is not silently represented as complete here.

## Success Criteria (Summary)

- Exact prior imports/skips and possible similar imported pictures inform—but never determine—each review decision.
- Unsupported comparison conditions are explicit and recoverable; originals and skipped-source previews remain protected.
- Format-5 catalogues migrate safely, and the full desktop review loop remains usable across restart and narrow layouts.

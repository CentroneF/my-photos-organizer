---
date: 2026-08-29T10:47:18+02:00
researcher: Codex (GPT-5)
git_commit: 300ca5999c45db9ae754eede9567b64987c94960
branch: show-duplicate-and-history-context
repository: CentroneF/my-photos-organizer
topic: "show-duplicate-and-history-context: how to find duplicate pictures and videos"
tags: [research, codebase, duplicate-detection, perceptual-hashing, review]
status: complete
last_updated: 2026-08-29
last_updated_by: Codex (GPT-5)
last_updated_note: "Added product direction: prioritize similar-picture context; retain exact hashes as a certainty/history signal."
---

# Research: Show duplicate and history context

**Date**: 2026-08-29T10:47:18+02:00  
**Researcher**: Codex (GPT-5)  
**Git Commit**: [300ca59](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/)  
**Branch**: `show-duplicate-and-history-context`  
**Repository**: `CentroneF/my-photos-organizer`

## Research Question

How should Photo Handler find exact duplicates and visually similar pictures and videos, then show safe prior-decision context while a person reviews an import item?

## Summary

Use two deliberately distinct signals:

1. **Exact duplicate** — compute and persist a streaming BLAKE3 digest of the file bytes. Equal digest means the exact same bytes, independent of filename, folder, or media type. This works for every supported photo and video format and satisfies the requirement to retain hashes for skipped files.
2. **Visually similar media** — compute a versioned perceptual image hash (initially 64-bit dHash) only for safely decodable images. Compare hashes with Hamming distance and present close candidates as *possible similar images*, never as an automatic decision. Perceptual hashing is intentionally tolerant of re-encoding/resizing/brightness changes but is not proof of identity.

For the first vertical slice, support visual similarity for JPEG, PNG, WebP, and GIF only. Keep exact matching and history for videos, HEIC, corrupt, or oversized images, and clearly say that visual comparison is unavailable for those files. Video similarity needs deterministic frame extraction plus a bundled decoder; the application does not currently have that capability, and adding FFmpeg or platform-specific codecs would add packaging and cross-platform risk. Do not infer video similarity from filenames, size, timestamps, or a single arbitrary thumbnail.

The feature belongs inside the existing `next_review_item` response so the preview, pending item, and context are derived from the same candidate. It must remain local, encrypted inside the SQLCipher catalogue, bounded, and advisory: Import and Skip stay enabled and explicit.

## How duplicate detection works

### Exact duplicates: cryptographic content digest

A content digest streams the entire file through a cryptographic hash function. Store the algorithm name/version and 32-byte digest; query already decided candidates for an equal digest. This gives an unambiguous byte-for-byte match, including videos and media the app cannot decode. BLAKE3 has an incremental Rust API suitable for buffered file reads and the same input produces the same digest across platforms; SHA-256 is also a valid conventional choice, but BLAKE3 is the better local throughput choice here. [BLAKE3 API](https://docs.rs/blake3/latest/blake3/struct.Hasher.html), [SHA-2 API](https://docs.rs/sha2/latest/sha2/).

Do not hash at source discovery: discovery is intentionally read-only and fast. Hash when a candidate is about to be shown, using a bounded buffer, then persist the result before returning review context. This makes the work visible and keeps a skipped file's fingerprint. Re-check the candidate's metadata before and after hashing; if it changed, return a recoverable “file changed, retry” state rather than matching stale content or deciding it.

### Similar images: perceptual hash and Hamming distance

Perceptual hashing reduces a decoded image to a compact visual signature. dHash compares adjacent grayscale pixels after resize; its Hamming distance is the number of differing bits. A small distance is a candidate for user review, not a duplicate assertion. Rust's `img_hash` exposes configurable image hashes and a `dist` operation; its documentation also explains that algorithm/hash-size combinations must match. Start with one documented algorithm/version and calibrate the threshold against committed non-personal fixtures rather than guessing a universal threshold. [img_hash documentation](https://docs.rs/img_hash/latest/img_hash/), [image-hash distance documentation](https://docs.rs/image_hasher/latest/image_hasher/struct.ImageHash.html).

Use pixel/file-size limits before decode and fail closed for decode errors. A perceptual hash may collide, and visually identical media can be missed after cropping, rotation, overlays, or large edits. Therefore the UI label must be “Possible similar image” with a score or qualitative confidence, not “Duplicate.”

### Videos

For exact video copies, byte hashing is enough. For visual similarity, a defensible future design samples deterministic normalized timestamps/keyframes, creates perceptual hashes per frame, then scores temporal alignment across multiple matching frames. FFmpeg can produce frame-level hashes, but that alone is not a near-duplicate detector and packaging a decoder is a new cross-platform product decision. [FFmpeg framehash formats](https://ffmpeg.org/ffmpeg-formats.html#framehash). Research literature likewise treats video copy detection as a temporal problem rather than a single image comparison. [Video-copy detection research](https://arxiv.org/abs/1911.09518).

Recommendation: explicitly scope video visual similarity out of the first S-04 plan phase; create a later framed decision covering decoder distribution, sample policy, runtime limits, and evaluation corpus. This still meets the current requirement for exact duplicate and prior-decision context for videos, but the PRD's MVP visual-similarity requirement means the later S-04 phase must add video support before the slice is declared fully complete.

## Detailed Findings

### Product and scope boundary

- S-04 requires both already handled items and possible similar media during review; it follows S-03 and is advisory, not automatic ([roadmap](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/context/foundation/roadmap.md#L93-L103)).
- FR-006 explicitly includes exact duplicates and visual similarity; FR-007 requires import/skip decisions, metadata, and hashes for skipped files ([PRD](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/context/foundation/prd.md#L59-L62)). The decision rule is suggestion-only ([PRD](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/context/foundation/prd.md#L83-L86)).
- S-03 intentionally persisted only minimal decisions and deferred hash/duplicate/similarity work, so no existing record can be retroactively matched unless its original is still available or its imported managed copy can be safely hashed.

### Existing data and native seams

- The encrypted catalogue is schema version 5 and already has review sessions, candidates, tags, and decisions ([library.rs](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/src-tauri/src/library.rs#L738)). Migrations are transactional and advance both schema and identity versions ([library.rs](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/src-tauri/src/library.rs#L874-L970)).
- Candidate rows retain source-relative path, size, modified time, media type, and decision; decision rows retain timestamp, destination, selected/original dates. This is enough to link a new candidate with prior imported *and* skipped history once fingerprints are added.
- `library::with_catalogue` opens the session-authenticated SQLCipher catalogue and provides the canonical library root ([library.rs](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/src-tauri/src/library.rs#L302-L338)). All fingerprint read/write/query work belongs behind this boundary.
- `next_review_item` is the correct single native aggregation point; its DTO is mirrored in Dioxus and its result already refreshes after Import or Skip ([review.rs](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/src-tauri/src/review.rs#L18-L41), [app.rs](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/src/app.rs#L633-L759)).
- The focused review card is intentionally compact and its explicit decisions must stay visible; render bounded informational context inside its details panel ([app.rs](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/src/app.rs#L1166-L1189)).

### Safe history presentation

Return at most 3–5 deterministic matches per category, ordered by `decided_at DESC, candidate_id`. A match DTO should include match kind, exact/similarity score, prior decision, display filename/relative path, handling date, tags, and dates. Do not emit absolute source paths. Only a prior **imported** item may receive an optional preview URL, and it must reuse the managed-copy containment and symlink checks in [`search::safe_preview_url`](https://github.com/CentroneF/my-photos-organizer/blob/300ca5999c45db9ae754eede9567b64987c94960/src-tauri/src/search.rs#L264-L304)). A skipped original is history metadata only: it must never be newly scoped to the webview.

## Recommended implementation shape

1. Bump catalogue format to 6. Add nullable candidate fields: `content_hash`, `content_hash_algorithm`, `perceptual_hash`, `perceptual_hash_algorithm`, and `perceptual_hash_status`; add an index for exact hash lookup. Seed the same schema for new libraries and test migration from v5.
2. Add a native fingerprint/context helper in `review.rs`. It resolves only the active canonical candidate, verifies it is a readable non-symlink file, streams BLAKE3, and persists the fingerprint through the encrypted connection. Decoded-image hashing uses an explicit algorithm/version and strict resource limits.
3. Extend `ReviewItem` with bounded `exact_matches`, `similar_matches`, and `history` (or one typed `matches` collection). Populate it in `next_review_item`; no new frontend filesystem permission or broad Tauri scope is needed.
4. Render short, accessible context panels: **Exact match — previously imported/skipped**; **Possible similar image**; **Visual comparison unavailable**. Keep Import/Skip unchanged and never preselect a decision.
5. Do not silently alter fingerprints for a candidate that already has a decision. Current reconciliation treats an existing source-relative path as the same candidate, even when content changes; decide and document whether a changed path becomes a new candidate before implementation.

## Verification strategy

- Equal bytes with different name/path/source return an exact historical match; equal filename/size but different bytes do not.
- A skipped candidate retains a content digest and later appears as a skipped exact match.
- A modified-between-discovery-and-hash candidate reports a recoverable state and creates no decision.
- Known near-image fixtures fall below the calibrated threshold; unrelated fixtures do not; exact matches are not duplicated in the similar list.
- HEIC, corrupt, oversized, and video files retain exact/history behavior while exposing no false visual claim.
- Version-5 migration preserves prior decisions. Context ordering and limits are deterministic, all paths remain local/encrypted, and no skipped-source URL is returned.
- Manual: context fits the review card; a user can still independently Import or Skip every item, and source bytes/names/paths remain unchanged.

## Architecture Insights

The app already has the right boundaries: native Rust owns source reads and catalogue writes; Dioxus receives serializable, bounded review DTOs; asset URLs are granted only after native validation. The important design choice is to preserve that separation. Fingerprints are catalogue metadata, not frontend file access; similarity is an explanation, not a workflow transition.

`search_library` intentionally returns imported records only, so it should not be expanded to expose skipped history. S-04 context belongs to the review flow, while it can safely reuse the stricter preview validation for managed imported copies.

## Historical Context

- `context/changes/review-and-decide-media/plan.md` deliberately deferred hashing, duplicate detection, visual similarity, and suggestion UI to S-04 while retaining durable minimal decisions.
- `context/changes/search-managed-library/plan.md` deliberately excludes skipped/pending records and duplicate/similarity search. Its managed-preview guard is reusable, but its product query boundary should remain unchanged.
- The current branch is already `show-duplicate-and-history-context`; the change folder was new and is now advanced from `new` to `preparing` with this research artifact.

## Related Research

No earlier `research.md` artifact exists for this change or an archived predecessor.

## Open Questions

1. What dHash/pHash algorithm version, decoded-pixel ceiling, and threshold should fixtures validate for this product's media mix?
2. Should a changed file at the same source-relative path create a new candidate rather than inherit a prior decision?
3. Which history fields are useful enough to show in the constrained review card, and should imported history include a managed-copy preview?
4. What is the acceptable wait/progress/cancellation experience for first-time hashing of large local videos?
5. What decoder/package strategy will support trustworthy video visual similarity on both macOS and Windows?

## Follow-up Research 2026-08-29T10:47:18+02:00

### Product direction: prioritize similar pictures

The user selected **similar-picture context** as the primary S-04 value. Exact byte equality is still worth persisting because it provides a certain, inexpensive local match and satisfies the skipped-file hashing requirement, but it should be a compact secondary signal rather than the focus of the review UI.

The implementation and UI should therefore:

- lead with **Possible similar pictures** when a perceptual-hash candidate is available, showing a bounded visual comparison and an understandable similarity label;
- show **Exact same file previously imported/skipped** as a concise certainty/history message;
- keep both signals advisory. Even an exact match must not auto-skip, auto-import, or delete anything: the reviewer continues to make the explicit decision required by the PRD;
- concentrate the initial perceptual-hashing work on supported still-image formats, where it offers the highest user value without adding a video decoder/distribution project.

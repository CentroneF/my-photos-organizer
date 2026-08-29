# Similar-picture context during import review — Implementation Plan

## Overview

Add trustworthy, bounded context to the existing one-item import review. Before a person decides, Photo Handler will retain an exact BLAKE3 fingerprint for the active file, show compact exact prior-decision history, and—where safe—show visually similar already-imported still images. Context remains advisory: Import and Skip remain independently available and originals are never changed.

## Current State Analysis

The encrypted catalogue is format 5 and persists candidates, decisions, tags, and imported managed destinations, but no fingerprints. `next_review_item` selects the active pending candidate and is the single DTO aggregation point for initial review and post-decision refresh. Search has an imported-only query plus a strict managed-copy preview guard, while the review card has deliberately persistent explicit decision controls.

The current resume key is only `(session_id, relative_path)`: a replacement at the same source-relative path would otherwise inherit prior metadata and a decision. The plan must therefore make changed bytes a new revision, not update an existing decided candidate in place.

## Desired End State

When reviewing a JPEG, PNG, WebP, or GIF, the person sees up to three safe managed-library thumbnails labelled **Possible similar picture**, plus a compact exact-match history when identical bytes were previously imported or skipped. Exact matching covers all supported media types; skipped history has metadata only and never grants a source preview. HEIC, video, corrupt, oversized, or unreadable images never claim that no match exists—they clearly state that visual comparison is unavailable while preserving exact-history behavior.

### Key Discoveries:

- `src-tauri/src/review.rs:188` already obtains the pending candidate before either decision and returns the DTO that the UI refreshes after Import or Skip.
- `src-tauri/src/library.rs:23`, `:738`, and `:874` define the format-5 constant, new-library schema, and transactional migration path.
- `src-tauri/src/search.rs:162` limits discovery to imported managed records, and `:264` validates managed-copy previews against the canonical root and rejects symlinks.
- `src/app.rs:633`, `:680`, and `:718` hydrate the same `ReviewItem` on start, skip, and import; `:1166` is the bounded review-card seam.

## What We're NOT Doing

- Automatic import, skip, deletion, source mutation, or preselected decisions based on any match.
- Visual video similarity, video frame extraction, FFmpeg distribution, HEIC visual decoding, or comparison of pending/skipped/source-only media.
- Broad frontend filesystem permissions, skipped-source URLs, full catalogue re-indexing, or changes to managed-library search scope.
- Claiming visual non-similarity when decode, resource bounds, or source stability prevents comparison.

## Implementation Approach

Advance the protected catalogue to format 6 with versioned nullable fingerprints and revision-aware candidates. In native Rust, fingerprint the active canonical candidate before building `ReviewItem`: stream a BLAKE3 content digest for every readable supported file, and compute a versioned 64-bit dHash only for bounded decodable still images. Verify stable file metadata before and after fingerprinting; persist successful results through SQLCipher, then query deterministic, capped context from the same catalogue connection.

Use exact equality as a secondary certainty/history signal across imported and skipped decisions. Restrict perceptual candidates to imported records with managed destinations, reuse the strict managed-preview containment check only for those matches, and send bounded serializable DTOs to Dioxus. The UI renders compact advisory panels before the existing actions and retains action visibility at desktop and narrow breakpoints.

## Critical Implementation Details

Fingerprint and persist the active candidate before returning its context, so a subsequent Skip retains its digest and a failed context calculation happens before Import can publish a copy. A file whose size or modified timestamp changes during hashing must return a recoverable changed/unavailable state without persisting a stale fingerprint or creating a decision. Do not use path-only resume reconciliation for changed bytes: append a replacement revision and keep the earlier decision as history.

## Phase 1: Exact history in the review flow

### Overview

Deliver the first complete, manually verifiable advisory signal: a newly reviewed file can show prior exact import/skip handling, and a changed same-path source becomes a separate pending revision rather than inheriting history.

### Changes Required:

#### 1. Protected catalogue format and revision-aware discovery

**Files**: `src-tauri/src/library.rs`, `src-tauri/src/review.rs`

**Intent**: Persist the durable exact-fingerprint data required for all media and prevent a different file at the same relative path from being silently treated as an already-decided candidate.

**Contract**: Bump the catalogue format from 5 to 6 in both new-library initialization and the transactional migration. Add nullable content-fingerprint value/algorithm fields and a lookup index, plus revision identity sufficient for one session/path to retain historical candidates while queueing a changed replacement. Migrate v5 without losing decisions, destinations, dates, or tags; legacy candidates remain eligible for context only after a safely readable managed/source file is fingerprinted. Discovery/resume compares the current file metadata with its stored revision and appends a pending revision when it changed; it never overwrites an existing decision.

#### 2. Exact fingerprint and context aggregation

**Files**: `src-tauri/Cargo.toml`, `src-tauri/src/review.rs`, `src-tauri/src/search.rs`

**Intent**: Compute a reliable byte-equality signal inside the native, encrypted boundary and return safe, bounded prior-decision context with the reviewed item.

**Contract**: Add `blake3` and stream readable non-symlink candidate bytes through its incremental hasher; record algorithm `blake3-256-v1` and the 32-byte digest before `next_review_item` returns. Check the recorded size/mtime before and after reading; a mismatch/read failure returns a recoverable item state and persists no stale result. Extend `ReviewItem` with a typed bounded context collection containing match kind, prior decision, filename/relative display path, decision date, tags, and optional managed preview URL. Exact queries include only decided candidates with the same algorithm/value, order by `decided_at DESC, candidate_id`, cap at three, and exclude the active candidate. Extract or share the `search::safe_preview_url` guard for imported destinations only; skipped matches never emit a URL or absolute source path.

#### 3. Exact-history panel and retained decisions

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Let the reviewer understand a certain prior match without turning it into an automated workflow.

**Contract**: Mirror the expanded native DTO and render a compact **Exact same file previously imported** or **Exact same file previously skipped** panel in `.review-details` before the existing action group. Imported history may use the guarded managed thumbnail; skipped history is textual metadata. Preserve the current independent Skip and Import controls, their unavailable-item behavior, and source-preview aspect ratio. Limit panel height/content and make it wrap/scroll safely so actions remain visible in the current two-column and narrow single-column layouts.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with format-5-to-6 migration, exact-match ordering/capping, imported-versus-skipped history, changed-path revision, and no-stale-fingerprint coverage.
- `cargo check --workspace` succeeds with synchronized native/Dioxus review DTOs and no new capability permission.
- Tests prove equal bytes under distinct names/sources match exactly; different bytes with equal name/size do not; skipped history yields no preview URL; source bytes, names, paths, and directory entries remain unchanged.

#### Manual Verification:

- Import a file, then review an identical copy from another source and see a compact imported exact-history message before choosing either decision.
- Skip a file, review an identical copy, and see skipped history without a preview or source-path exposure.
- Replace a previously decided same-relative-path source file, resume review, and see it as a newly pending item; Import and Skip remain explicit and visible.

**Implementation Note**: After automated checks pass, pause for human confirmation that exact-history context is useful, bounded, and does not alter the independent decision flow.

---

## Phase 2: Similar-picture context for supported still images

### Overview

Deliver visual comparison for safely decodable still images with a calibrated, non-assertive signal and compact managed-library thumbnails.

### Changes Required:

#### 1. Bounded perceptual hashing and imported-only similarity query

**Files**: `src-tauri/Cargo.toml`, `src-tauri/src/library.rs`, `src-tauri/src/review.rs`, `src-tauri/src/search.rs`

**Intent**: Add a versioned visual signature that finds useful near-image candidates without conflating similarity with identity or widening the comparison universe.

**Contract**: Add a Rust image decoder and perceptual-hash dependency compatible with JPEG, PNG, WebP, and GIF. Persist a nullable `dhash-64-v1` value and an explicit status (`available`, `unsupported`, `decode_failed`, `resource_limited`, or `changed`) for the active candidate; use a documented decoded-pixel ceiling of 40 million pixels before decoding. Compare only same-version perceptual hashes for records whose decision is `imported` and whose destination is present; calculate Hamming distance, exclude exact content matches from the similar list, order by ascending distance then `decided_at DESC, candidate_id`, and cap at three. Use distance 10 or below only after committed fixtures demonstrate its expected near-versus-unrelated behavior; store the threshold alongside the algorithm version so recalibration is a migration/contract change, not a silent reinterpretation.

#### 2. Similarity DTO and advisory review UI

**Files**: `src-tauri/src/review.rs`, `src/app.rs`, `assets/styles.css`

**Intent**: Make visual context easy to judge while stating the limit of the evidence truthfully.

**Contract**: Add a separate bounded `similar_matches` DTO collection plus a `visual_comparison_state`/message to `ReviewItem`. Render **Possible similar pictures** ahead of exact history, with up to three guarded imported thumbnails, filename, handling date, tags, and qualitative similarity labels; do not show a raw distance or call any result a duplicate. Render a concise unavailable message for video, HEIC, corrupt, oversized, or decode-failed candidates. Keep decisions enabled during the inline checking state and on unavailable results; context never arrives after the item has advanced.

#### 3. Non-personal fixture calibration and behavior tests

**Files**: `src-tauri/src/review.rs`, `src/app.rs`, `assets/styles.css`, new non-personal fixtures under `src-tauri/tests/fixtures/` if required

**Intent**: Make the initial threshold, supported-format boundary, and responsive presentation repeatable rather than subjective.

**Contract**: Commit only generated or openly licensed, non-personal small image fixtures: one base image, benign resize/re-encode/brightness variants, and clearly unrelated controls. Tests must lock the selected `dhash-64-v1` threshold at 10, prove close variants appear while controls do not, and prove identical bytes are listed only as exact history. Add source/CSS assertions for the advisory labels, unavailable state, three-item bound, and retained Import/Skip actions.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with supported-format hashing, fixture calibration, imported-only similarity, exact/similar de-duplication, deterministic ordering, resource-limit, decode-failure, and video/HEIC unavailable-state tests.
- `cargo check --workspace` succeeds with pinned image/perceptual-hash dependencies recorded in `Cargo.lock`.
- UI source/layout tests prove similar context is bounded, advisory, responsive, and cannot hide or disable explicit decisions.

#### Manual Verification:

- Import a supported image, then review a resized/re-encoded similar image and see at most three clearly labelled managed-picture comparisons before deciding.
- Review a supported but unrelated image and see no false similar-picture result; review HEIC/video and see an honest unavailable message while exact history still works.
- Resize the desktop review view to the narrow breakpoint and confirm previews/context remain legible while both decisions remain reachable.

**Implementation Note**: After automated checks pass, pause for human confirmation using non-personal media that similarity labels are understandable and the card stays usable.

---

## Phase 3: Recovery, migration, and safety hardening

### Overview

Complete the vertical slice with recoverable file-change behavior, legacy-catalogue confidence, and desktop end-to-end verification of the advisory boundary.

### Changes Required:

#### 1. Fingerprint failure lifecycle and regression coverage

**Files**: `src-tauri/src/review.rs`, `src-tauri/src/library.rs`, `src/app.rs`

**Intent**: Ensure a transient or unsafe comparison condition is never converted into a false match, stale record, or implicit decision.

**Contract**: Cover and expose changed-during-hash, unreadable, and write/query failure states as recoverable review feedback. Persist only confirmed content/perceptual values, preserve pending status on failure, and permit the person to make an explicit decision where the existing review contract permits it. Verify a v5 catalogue migrates transactionally on unlock and remains readable after restart; no migration failure may modify source media or expose catalogue paths to the webview.

#### 2. Full review-flow verification and documentation alignment

**Files**: `src-tauri/src/review.rs`, `src-tauri/src/library.rs`, `src/app.rs`, `context/foundation/roadmap.md`

**Intent**: Prove the change is an end-to-end review capability and record the intentional remaining video-similarity boundary.

**Contract**: Add end-to-end-style native tests that create a protected library, import and skip seed files, restart/unlock, and inspect the next review item’s exact/similar/unavailable context without mutation. Update S-04 roadmap status/notes only when all phase criteria pass; explicitly record that visual video similarity remains a future separately framed decoder/distribution decision rather than implying FR-006 coverage for video.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes from a clean checkout and covers v5 migration, restart/unlock persistence, changed-during-hash recovery, source preservation, and preview containment/symlink rejection.
- `cargo tauri build` succeeds on the development platform with the new pinned dependencies and no generated build artifacts added to Git.

#### Manual Verification:

- In the desktop app, complete an import/skip/restart/review loop with exact and similar image context, then verify every original remains untouched.
- Deliberately make a candidate unavailable or replace it before review; receive a clear recoverable message rather than a false “no match” result or an automatic decision.
- Confirm skipped records remain historical only, imported comparison previews remain inside the managed library, and no card grants broad access to an arbitrary source file.

**Implementation Note**: After automated checks pass, pause for final human confirmation of the full desktop flow before marking this change implemented.

---

## Testing Strategy

### Unit Tests:

- BLAKE3 digest persistence, content equality, changed-file pre/post metadata checks, and deterministic exact-match selection.
- Format-5 migration, revision append behavior, nullable legacy fingerprint behavior, and imported-only/symlink-safe previews.
- dHash fixture calibration, algorithm/version filtering, Hamming-distance limit, resource/decode state, and exact/similar de-duplication.

### Integration Tests:

- Protected catalogue setup → import/skip seed decisions → lock/reopen → new review DTO returns correct history and similarity.
- Source replacement at the same relative path → resumed session contains a new pending revision and preserves the old decision.

### Manual Testing Steps:

1. Import a fixture image, then review an identical copy and confirm exact imported context without any automatic action.
2. Review a visually similar supported image and an unrelated control; confirm only the near image has bounded managed previews.
3. Skip an identical source, restart, and confirm it is text-only exact history.
4. Review video, HEIC, corrupt, and oversized samples; confirm visual comparison is unavailable, not negative, and explicit decisions remain available.

## Performance Considerations

Hash only the item currently entering review; never scan all source files during discovery or launch. Stream BLAKE3 rather than loading whole files, cap decoded image pixels at 40 million, use indexed exact lookups, and limit each context category to three items. The initial inline checking state is acceptable because it is bounded to one local candidate and never delays a decision after completion/failure.

## Migration Notes

Format 6 introduces nullable fingerprint/status fields so existing format-5 decisions remain valid history without fabricated hashes. Migration runs in the existing SQLCipher transaction and updates both schema-version records only after every DDL step succeeds. Rollback before release is restoring the pre-migration protected catalogue backup; after a released migration, retain backward-read compatibility through nullable fields rather than attempting destructive downgrade.

## References

- Frame: `context/changes/show-duplicate-and-history-context/frame.md`
- Research: `context/changes/show-duplicate-and-history-context/research.md`
- Existing review flow: `src-tauri/src/review.rs:188-305`, `src/app.rs:633-758`, `src/app.rs:1166-1189`
- Catalogue migration: `src-tauri/src/library.rs:23-24`, `src-tauri/src/library.rs:738-739`, `src-tauri/src/library.rs:874-970`
- Managed-preview guard: `src-tauri/src/search.rs:162-211`, `src-tauri/src/search.rs:264-304`
- BLAKE3 incremental hashing: <https://docs.rs/blake3/latest/blake3/struct.Hasher.html>

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands.

### Phase 1: Exact history in the review flow

#### Automated

- [x] 1.1 `cargo test --workspace` passes with migration, exact history, revision, and source-preservation coverage.
- [x] 1.2 `cargo check --workspace` succeeds with synchronized DTOs and no new capability permission.
- [x] 1.3 Exact-match tests prove equality/difference handling, capped order, and no skipped preview URL.

#### Manual

- [x] 1.4 Imported and skipped exact-history contexts are correct, bounded, and leave decisions explicit.
- [x] 1.5 A changed same-path file is presented as newly pending without modifying its source.

### Phase 2: Similar-picture context for supported still images

#### Automated

- [ ] 2.1 `cargo test --workspace` passes with supported-format hashing, fixture calibration, safety states, and imported-only similarity coverage.
- [ ] 2.2 `cargo check --workspace` succeeds with pinned dependencies and synchronized DTOs.
- [ ] 2.3 UI tests prove bounded advisory context, unavailable state, responsiveness, and retained actions.

#### Manual

- [ ] 2.4 Similar supported images show compact managed comparisons; unrelated images do not.
- [ ] 2.5 HEIC/video unavailable states are honest and the narrow review layout retains both actions.

### Phase 3: Recovery, migration, and safety hardening

#### Automated

- [ ] 3.1 `cargo test --workspace` passes from a clean checkout with migration/restart/recovery and containment coverage.
- [ ] 3.2 `cargo tauri build` succeeds without generated artifacts added to Git.

#### Manual

- [ ] 3.3 The desktop import/skip/restart/review loop preserves originals and keeps history/preview boundaries safe.
- [ ] 3.4 Unavailable or changed candidates show recoverable feedback without false matches or automatic decisions.

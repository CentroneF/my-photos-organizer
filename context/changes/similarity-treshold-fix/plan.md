# Similarity Threshold Presets Implementation Plan

## Overview

Add a library-scoped similarity preference so reviewers can choose Strict (8), Balanced (10), Broad (14), or Very Broad (16). The selected threshold applies immediately to existing compatible dHash records without re-importing or re-hashing media.

## Current State Analysis

The review flow writes a fixed `dhash-64-v1` value and the constant threshold `10` to each active candidate. The similarity query incorrectly treats that historical threshold as a compatibility requirement, then applies the same fixed threshold to the Hamming-distance result. There is no settings model, native settings command, or UI control for this behavior.

## Desired End State

An unlocked protected library retains one selected similarity preset, defaulting to Balanced (10). A reviewer can select any of the four labelled presets in the review workspace and see the active choice. The next comparison uses that selected threshold against every eligible `dhash-64-v1` record, including legacy rows whose stored threshold is `NULL` or differs from the current setting. Originals remain untouched and unsupported/pending/skipped media retain their existing non-comparable behavior.

### Key Discoveries:

- The current fixed value is `SIMILARITY_THRESHOLD: u32 = 10`; it is saved in `src-tauri/src/review.rs:347` and used both in the SQL predicate and Hamming-distance filter at `src-tauri/src/review.rs:1040-1066`.
- A dHash value’s compatibility is defined by `dhash-64-v1`, not the threshold used during a previous review. The v8 migration added the per-candidate threshold as nullable data: `src-tauri/src/library.rs:742,958-961`.
- Catalogue migrations already update both schema-version records transactionally in `src-tauri/src/library.rs:894-1012`.
- The Dioxus review workspace and source-contract tests live in `src/app.rs:430-920,1498-1665`; no generic settings UI exists.

## What We're NOT Doing

- Comparing pending, skipped, unsupported, corrupt, oversized, or source-only media with visual hashes.
- Rehashing/backfilling the catalogue, changing the dHash algorithm, or adding arbitrary 0–64 numeric input.
- Exposing raw Hamming distances, changing exact-match behavior, or granting frontend filesystem access.
- Modifying, moving, or deleting source originals.

## Implementation Approach

Persist a single validated threshold preference in the encrypted catalogue with default value 10. Native review code reads that preference for both persisting the current candidate’s diagnostic provenance and filtering Hamming distances, while the candidate query matches only compatible algorithm/version and non-null hash data. Expose a thin native get/set boundary to Dioxus and render the four labelled presets in the review workspace. Cover default/migration behavior, setting validation, legacy compatibility, and the calibrated distance boundary with generated non-personal test images.

## Critical Implementation Details

The selected threshold changes comparison policy, not dHash compatibility: the SQL query must never require `perceptual_hash_threshold` to equal the current setting. Keep the historic column for diagnostic provenance, but compare every eligible `dhash-64-v1` value and apply the active threshold only after calculating Hamming distance.

## Phase 1: Library-Scoped Similarity Presets

### Overview

Deliver the complete reviewer-facing preference: a durable four-choice setting, immediate broader matching for existing compatible imports, and regression coverage proving higher recall does not widen unsupported comparison boundaries.

### Changes Required:

#### 1. Encrypted catalogue preference and migration

**File**: `src-tauri/src/library.rs`

**Intent**: Store an authenticated library-owned threshold rather than treating a per-candidate historic value as global behavior.

**Contract**: Bump the catalogue format and create/migrate a one-row settings record with default threshold 10. Export serializable get/update request and result types that accept only 8, 10, 14, or 16, reject all other values with a structured error, and preserve existing catalogues and decisions on migration/reopen.

#### 2. Active threshold matching and native command boundary

**Files**: `src-tauri/src/review.rs`, `src-tauri/src/lib.rs`

**Intent**: Make the active library preference control similarity recall for all compatible imported hashes.

**Contract**: Resolve the setting in the native review path, persist it only as current-candidate provenance, and pass it to the Hamming-distance filter. Similar-match eligibility remains imported managed, non-superseded, non-exact, and algorithm-compatible with a non-null dHash; it no longer filters by historic `perceptual_hash_threshold`. Register synchronized get/set Tauri commands without adding capabilities or frontend file access. Include the active threshold in the review/settings response needed by the UI.

#### 3. Preset control in the review workspace

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Let reviewers choose an understandable recall tradeoff while seeing which choice is active.

**Contract**: Load the library preference when entering review and render four accessible mutually exclusive controls: Strict (8), Balanced (10), Broad (14), and Very Broad (16). Updating a preset calls the native command, retains the current review item and tags, refreshes its comparison context once, and surfaces native errors without silently changing the selected control. Style the group for the existing full-window review layout and narrow breakpoint without hiding Import, Skip, or comparison controls.

#### 4. Calibrated safety and regression coverage

**Files**: `src-tauri/src/library.rs`, `src-tauri/src/review.rs`, `src/app.rs`, `assets/styles.css`

**Intent**: Lock the preset contract and prove a higher threshold expands only eligible dHash matches.

**Contract**: Test fresh/migrated default 10, valid preset persistence/reopen, invalid-value rejection, and immediate setting use. Extend similarity fixtures with values at distances 8, 10, 14, 16, and 17; legacy `NULL`/different stored thresholds; incompatible algorithms; exact fingerprints; skipped/no-destination/replaced candidates. Assert Broad includes through 14, Very Broad includes through 16, and distance 17 remains excluded with deterministic newest-first ordering. Retain unsupported/HEIC/resource-limit boundaries and add UI source/CSS contracts for labels, selected state, command hooks, refresh, error retention, and responsive layout.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with catalogue migration/reopen, preset validation, legacy hash compatibility, calibrated distance boundaries, and UI contract coverage.
- `cargo tauri build --bundles app` succeeds; no DMG is produced or added to Git.

#### Manual Verification:

- In an unlocked library, switch among Strict, Balanced, Broad, and Very Broad; reopen the library and confirm the selected preset persists.
- Review the verified classroom-photo set at Broad and Very Broad; confirm the intended related images appear while unrelated controls remain absent at the documented boundary.
- Change a preset while reviewing an item; confirm its context refreshes without losing tags/date or disabling Import, Skip, or Substitute, and confirm every source original remains unchanged.

**Implementation Note**: After automated verification passes, pause for human confirmation of the preference and real-photo comparison workflow before declaring the change complete.

## Testing Strategy

### Unit Tests:

- Catalogue default, migration, validation, persistence, and reopen behavior for each allowed threshold.
- Similar-match eligibility remains algorithm/version-based and imported-only, regardless of historic threshold provenance.
- Hamming-distance boundaries: 8/10/14/16 included by their matching presets and 17 excluded by Very Broad.

### Integration Tests:

- Protected library setup or migration → select a preset → reopen → load the next review item with the retained effective threshold.
- Legacy/different-threshold imported records remain comparable when their `dhash-64-v1` value is present, while skipped, replaced, exact, and unavailable candidates remain excluded.

### Manual Testing Steps:

1. Start a review, choose each preset, and confirm its active state is clear and persists after locking/reopening the library.
2. Use the verified classroom photos and unrelated controls to validate the expected Broad (14) and Very Broad (16) results.
3. Change the preset during an active review, then Import, Skip, and Substitute as applicable; verify source bytes/names/locations do not change.

## Performance Considerations

Threshold changes add no per-comparison cost beyond the existing 64-bit XOR and popcount. Higher presets may return more matches, so the existing scrollable similarity list remains the render bound; no rehash or full-library scan is introduced.

## Migration Notes

The new library setting defaults existing catalogues to Balanced (10). Historic per-candidate threshold values remain untouched for provenance but are not a compatibility filter; no destructive migration, source scan, or media rewrite occurs.

## References

- Frame: `context/changes/similarity-treshold-fix/frame.md`
- Input discussion: `context/changes/similarity-treshold-fix/threshold-discussion.md`
- Similarity matching: `src-tauri/src/review.rs:21-23,345-350,949-1066`
- Catalogue migrations: `src-tauri/src/library.rs:738-746,894-1012`
- Review UI/tests: `src/app.rs:430-920,1498-1665`; `assets/styles.css:91-131`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Library-Scoped Similarity Presets

#### Automated

- [ ] 1.1 Add the encrypted library threshold preference, migration, and validation contract
- [ ] 1.2 Apply the active preset to native similarity matching and register its Tauri commands
- [ ] 1.3 Add accessible Strict, Balanced, Broad, and Very Broad controls to the review workspace
- [ ] 1.4 Run `cargo test --workspace` for preference, compatibility, calibration, and UI contracts
- [ ] 1.5 Run `cargo tauri build --bundles app` without generating a DMG

#### Manual

- [ ] 1.6 Verify preset persistence, classroom-photo recall, retained review actions, and source preservation

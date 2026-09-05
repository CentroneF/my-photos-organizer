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

Deliver the complete library-settings preference: a durable four-choice setting outside the review decision workspace, immediate broader matching for existing compatible imports, and regression coverage proving higher recall does not widen unsupported comparison boundaries.

### Changes Required:

#### 1. Encrypted catalogue preference and migration

**File**: `src-tauri/src/library.rs`

**Intent**: Store an authenticated library-owned threshold rather than treating a per-candidate historic value as global behavior.

**Contract**: Bump the catalogue format and create/migrate a one-row settings record with default threshold 10. Export serializable get/update request and result types that accept only 8, 10, 14, or 16, reject all other values with a structured error, and preserve existing catalogues and decisions on migration/reopen.

#### 2. Active threshold matching and native command boundary

**Files**: `src-tauri/src/review.rs`, `src-tauri/src/lib.rs`

**Intent**: Make the active library preference control similarity recall for all compatible imported hashes.

**Contract**: Resolve the setting in the native review path, persist it only as current-candidate provenance, and pass it to the Hamming-distance filter. Similar-match eligibility remains imported managed, non-superseded, non-exact, and algorithm-compatible with a non-null dHash; it no longer filters by historic `perceptual_hash_threshold`. Register synchronized get/set Tauri commands without adding capabilities or frontend file access. Include the active threshold in the review/settings response needed by the UI.

#### 3. Dedicated Library Settings screen

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Keep review focused on the current import decision while giving library owners a clear, persistent place to manage matching behavior.

**Contract**: Add a `Library settings` destination from the library-list header. Its focused screen shows the active protected-library folder path and four accessible mutually exclusive controls: Strict (8), Balanced (10), Broad (14), and Very Broad (16). Load the preference each time the screen opens; updating a preset calls the native command, retains the selected control only on success, and surfaces native errors. Remove all threshold controls and threshold-refresh behavior from Review; its next loaded item reads the persisted preference natively. Style the screen and narrow breakpoint without crowding library-list actions or hiding review controls.

#### 4. Calibrated safety and regression coverage

**Files**: `src-tauri/src/library.rs`, `src-tauri/src/review.rs`, `src/app.rs`, `assets/styles.css`

**Intent**: Lock the preset contract and prove a higher threshold expands only eligible dHash matches.

**Contract**: Test fresh/migrated default 10, valid preset persistence/reopen, invalid-value rejection, and native setting use. Extend similarity fixtures with values at distances 8, 10, 14, 16, and 17; legacy `NULL`/different stored thresholds; incompatible algorithms; exact fingerprints; skipped/no-destination/replaced candidates. Assert Broad includes through 14, Very Broad includes through 16, and distance 17 remains excluded with deterministic newest-first ordering. Retain unsupported/HEIC/resource-limit boundaries and add UI source/CSS contracts for the Library Settings entry point, protected path, labels, selected state, command hooks, error retention, review-control absence, and responsive layout.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with catalogue migration/reopen, preset validation, legacy hash compatibility, calibrated distance boundaries, and UI contract coverage.
- `cargo tauri build --bundles app` succeeds; no DMG is produced or added to Git.

#### Manual Verification:

- From the library list, open Library settings; confirm the protected-library path and switch among Strict, Balanced, Broad, and Very Broad. Reopen the library and confirm the selected preset persists.
- Review the verified classroom-photo set at Broad and Very Broad; confirm the intended related images appear while unrelated controls remain absent at the documented boundary.
- Confirm Review contains no threshold picker and that Import, Skip, Substitute, tags, dates, and source originals remain unchanged.

**Implementation Note**: After automated verification passes, pause for human confirmation of the preference and real-photo comparison workflow before declaring the change complete.

## Testing Strategy

### Unit Tests:

- Catalogue default, migration, validation, persistence, and reopen behavior for each allowed threshold.
- Similar-match eligibility remains algorithm/version-based and imported-only, regardless of historic threshold provenance.
- Hamming-distance boundaries: 8/10/14/16 included by their matching presets and 17 excluded by Very Broad.

### Integration Tests:

- Protected library setup or migration → select a preset in Library settings → reopen → load the next review item with the retained effective threshold.
- Legacy/different-threshold imported records remain comparable when their `dhash-64-v1` value is present, while skipped, replaced, exact, and unavailable candidates remain excluded.

### Manual Testing Steps:

1. From the library list, open Library settings, confirm the protected path, choose each preset, and confirm its active state is clear and persists after locking/reopening the library.
2. Use the verified classroom photos and unrelated controls to validate the expected Broad (14) and Very Broad (16) results.
3. Confirm Review has no threshold picker, then Import, Skip, and Substitute as applicable; verify source bytes/names/locations do not change.

## Performance Considerations

Threshold changes add no per-comparison cost beyond the existing 64-bit XOR and popcount. Higher presets may return more matches, so the existing scrollable similarity list remains the render bound; no rehash or full-library scan is introduced.

## Migration Notes

The new library setting defaults existing catalogues to Balanced (10). Historic per-candidate threshold values remain untouched for provenance but are not a compatibility filter; no destructive migration, source scan, or media rewrite occurs.

## References

- Frame: `context/changes/similarity-treshold-fix/frame.md`
- Input discussion: `context/changes/similarity-treshold-fix/threshold-discussion.md`
- Similarity matching: `src-tauri/src/review.rs:21-23,345-350,949-1066`
- Catalogue migrations: `src-tauri/src/library.rs:738-746,894-1012`
- Library/settings UI/tests: `src/app.rs:430-920,1348-1568,1677-1800`; `assets/styles.css:91-135`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Library-Scoped Similarity Presets

#### Automated

- [x] 1.1 Add the encrypted library threshold preference, migration, and validation contract
- [x] 1.2 Apply the active preset to native similarity matching and register its Tauri commands
- [ ] 1.3 Move accessible Strict, Balanced, Broad, and Very Broad controls into Library Settings
- [ ] 1.4 Run `cargo test --workspace` for preference, compatibility, calibration, and UI contracts
- [ ] 1.5 Run `cargo tauri build --bundles app` without generating a DMG

#### Manual

- [ ] 1.6 Verify Library Settings persistence, classroom-photo recall, review focus, and source preservation

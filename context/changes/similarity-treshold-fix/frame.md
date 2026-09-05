# Frame Brief: Similarity threshold fix

> Framing step before /10x-plan. This document separates the observed missed
> similarity from the initially assumed threshold fix.

## Reported Observation

Three classroom photos were visually related but were not surfaced as similar by the app.

## Initial Framing (preserved)

- **User's stated cause or approach**: The fixed 64-bit dHash threshold of 10 may be too strict; storing and querying that threshold with each hash would also make a future setting incompatible with older hashes.
- **User's proposed direction**: Make the threshold configurable, compare by algorithm compatibility instead, and calibrate it with fixtures.
- **Pre-dispatch narrowing**: At least one observed photo was pending, skipped, old, or displayed visual-comparison-unavailable state.

## Dimension Map

The observation could originate at any of these dimensions:

1. **Candidate eligibility** — only prior imported, non-replaced candidates with a managed destination are queried.
2. **Persisted compatibility data** — algorithm, stored threshold, and fingerprint values must all satisfy the query.
3. **dHash recall** — 9×8 grayscale dHash can place related reframed or moving-subject photos beyond distance 10.
4. **Hash input availability** — pending, skipped, legacy, HEIC, oversized, corrupt, or orientation-sensitive images may have no usable visual hash.

## Hypothesis Investigation

| Hypothesis | Evidence | Verdict |
| --- | --- | --- |
| Eligibility excludes one or more observed photos | The query accepts only imported, managed, non-replaced decisions; pending and skipped candidates are intentionally absent. `src-tauri/src/review.rs:1040-1047`; user reported at least one non-normal candidate state. | STRONG |
| Stored threshold equality caused the present miss | Newly opened hashable items are saved with threshold 10 and queried with the same constant. `src-tauri/src/review.rs:22,347,1040-1045`. Legacy `NULL` values can be excluded because the v8 migration only adds a nullable column. `src-tauri/src/library.rs:954-961`. | WEAK for this incident; STRONG future compatibility risk |
| Threshold 10 rejects visually related classroom photos | Distances above 10 are discarded. `src-tauri/src/review.rs:1061-1066`. The implementation resizes the entire image to 9×8 grayscale before comparison. `src-tauri/src/review.rs:949-1003`. No real-photo distance, crop, rotation, or exposure calibration exists. | WEAK |
| Input availability prevents a comparison | HEIC is discovered as an image but is unsupported by the dHash path; the hash also rejects oversized and decode-failed sources. `src-tauri/src/review.rs:949-985,1216-1221`; `src-tauri/Cargo.toml:38`. | STRONG alternative |

## Narrowing Signals

- The leading concern is missing related photos, not merely exposing a setting.
- At least one observed photo was not an ordinary already-imported, hashable candidate, ruling candidate eligibility and hash availability in ahead of threshold tuning.

## Cross-System Convention

Similarity thresholds only control recall after both records are compatible and eligible for comparison. This codebase follows that boundary: native review owns hash creation and the catalogue query only exposes safe imported managed copies. The existing `threshold-discussion.md` also records that pending photos are not compared with one another and old imports are not backfilled.

## Reframed Problem Statement

> **The actual problem to plan around is**: visually related photos can be absent before the dHash threshold is evaluated because their review state or visual-hash availability makes them ineligible for comparison.

The threshold of 10 may still limit recall for eligible photos, but the observed incident cannot establish that without verifying the photos' formats, decision states, stored metadata, and pairwise distances. Planning only a configurable threshold risks shipping a control that cannot surface the reported photos. The persisted threshold equality remains a separate compatibility defect to address if a runtime setting is introduced.

## Confidence

- **HIGH** — the user’s narrowed observation matches the query’s documented eligibility boundary, and independent code inspection found multiple availability paths that bypass distance comparison.

## What Changes for /10x-plan

The plan should first make visual-comparison eligibility observable and test the reported media states, then decide whether calibrated threshold configuration is needed for eligible pairs. It should retain the compatibility concern as a requirement only if the product exposes runtime threshold selection.

## References

- Source files: `src-tauri/src/review.rs:22,347,949-1066,1216-1221`; `src-tauri/src/library.rs:954-961`; `src-tauri/Cargo.toml:38`
- Input discussion: `context/changes/similarity-treshold-fix/threshold-discussion.md`
- Related review decision: `context/changes/show-duplicate-and-history-context/reviews/impl-review.md:64-70`
- Investigation tasks: `/root/threshold_query`, `/root/hash_sensitivity`

# Similarity Threshold Presets — Plan Brief

> Full plan: `context/changes/similarity-treshold-fix/plan.md`
> Frame brief: `context/changes/similarity-treshold-fix/frame.md`

## What & Why

Add a library-scoped choice of Strict (8), Balanced (10), Broad (14), and Very Broad (16) for visual similarity matching. The current threshold of 10 missed verified related classroom photos; the setting improves recall while keeping the dHash algorithm and source-media safety boundary unchanged.

## Starting Point

Review uses a fixed threshold of 10 and persists it on each dHash record. The matching query incorrectly treats that historical value as a compatibility requirement, which would exclude otherwise compatible records when policy changes.

## Desired End State

An unlocked library remembers its selected preset and reviewers can change it directly in the review workspace. Existing compatible `dhash-64-v1` imports are considered immediately at the new threshold, without re-importing or re-hashing media.

## Key Decisions Made

| Decision | Choice | Why | Source |
| --- | --- | --- | --- |
| Preference scope | Per protected library | Keeps behavior stable across devices for one catalogue. | Plan |
| Presets | 8, 10, 14, 16 | Offers understandable recall levels without raw hash tuning. | Plan |
| Compatibility | Algorithm/version only | Historic thresholds are provenance, not hash compatibility. | Frame |
| Calibration | Generated tests + real-photo check | Keeps regression coverage shareable while validating the observed use case. | Plan |

## Scope

**In scope:** encrypted preference storage; migration; native commands; review preset controls; compatible legacy matching; calibration and UI tests.

**Out of scope:** arbitrary numeric thresholds, rehash/backfill, pending/skipped/HEIC/video visual comparison, source-media mutation, and raw distance display.

## Architecture / Approach

The encrypted catalogue owns the selected threshold. Native review matching reads it at comparison time and compares eligible `dhash-64-v1` values without requiring their historical threshold to match. Dioxus reads and updates the setting through narrow Tauri commands and refreshes the current item’s comparison context.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Library-scoped similarity presets | Persistent four-choice control and immediate compatible matching | Higher recall can introduce false positives |

**Prerequisites:** verified classroom-photo set for manual validation; existing protected-catalogue test setup.
**Estimated effort:** ~1 focused implementation session.

## Open Risks & Assumptions

- Very Broad (16) can increase false positives; generated boundary tests and manual controls must verify the tradeoff.
- Unsupported or ineligible media remains outside visual comparison regardless of the selected preset.

## Success Criteria (Summary)

- Presets persist per library and safely apply to existing compatible hashes.
- Broad and Very Broad surface the verified related images at their calibrated boundaries.
- Review actions and source-original protection remain unchanged.

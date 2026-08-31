# Frame Brief: Similar picture context during import review

> Framing step before `/10x-plan`. This document captures what is *actually*
> at issue, separated from what was initially assumed.

## Reported Observation

“When i import a new picture i should be able to see if any similar picture was imported already.”

## Initial Framing (preserved)

- **User's stated cause or approach**: Exact duplicate detection is obvious; visual similarity is the information that matters during review.
- **User's proposed direction**: Prioritize similar-picture context, retaining exact equality as a secondary certainty/history signal.
- **Pre-dispatch narrowing**: The relevant comparison is a new import candidate against pictures already imported into the managed library.

## Dimension Map

The observation could originate at any of these dimensions:

1. **Visual signal** — the catalogue has no visual representation by which a new picture can be compared with a prior image. ← initial framing
2. **Comparison universe** — “already imported” could be incorrectly mixed with skipped, pending, or source-only records.
3. **Review-time context** — a similarity result could arrive after a decision, or replace the person’s explicit decision rather than informing it.
4. **Decode coverage** — an unsupported image could be mistaken for a negative similarity result rather than an unavailable comparison.

## Hypothesis Investigation

| Hypothesis | Evidence | Verdict |
| --- | --- | --- |
| No visual signal links a new picture to a prior image | FR-006 requires possible similar media during review; the live catalogue has decisions and media metadata but no visual fingerprint ([PRD](../../foundation/prd.md), [library.rs](../../../src-tauri/src/library.rs:738)). | **STRONG** |
| Prior imported media is the correct similarity universe | User clarification; imports alone receive a managed destination, and the established library query is imported-only ([review.rs](../../../src-tauri/src/review.rs:280), [search.rs](../../../src-tauri/src/search.rs:171)). | **STRONG** |
| The result must be review-time advisory context | `next_review_item` precedes either explicit decision, and Import/Skip are independent handlers ([app.rs](../../../src/app.rs:633), [app.rs](../../../src/app.rs:680), [PRD](../../foundation/prd.md)). | **STRONG** |
| Decode coverage changes the truthfulness of the result | Current source support includes HEIC but no image-decoding/similarity dependency exists ([review.rs](../../../src-tauri/src/review.rs:511), [Cargo.toml](../../../src-tauri/Cargo.toml:20)). | **STRONG** |

## Narrowing Signals

- The user named the target relationship precisely: **a new picture** compared with one **already imported**.
- Imported records have managed copies that can be safely identified; skipped records are still required historical evidence but do not have a managed destination or a safe preview boundary.
- The existing review flow already obtains the pending candidate before presenting Import or Skip, so this is a context gap rather than a new decision workflow.

## Cross-System Convention

The completed search slice deliberately limits managed-library discovery to imported records and excludes skipped, pending, and source-only candidates (`context/changes/search-managed-library/plan.md`). That convention independently supports an imported-managed-library reference set for similar pictures. The S-03 plan separately deferred visual similarity and hashing to S-04, confirming that the current gap is intentional rather than a regression.

## Reframed (or Confirmed) Problem Statement

> **The actual problem to plan around is**: During review of a new image, Photo Handler lacks a trustworthy, bounded way to show visually similar images that are already safely imported in the managed library, while preserving explicit user decisions and separate skip-history records.

The initial framing was correct, with one necessary boundary: similarity applies to prior imported managed images, not to every historical or source record. Prior skipped decisions remain separate metadata/history because the PRD requires them, but they must not be presented as comparable managed-library pictures. A missing or unsupported visual comparison must be conveyed as unavailable, not “no similar picture exists.”

## Confidence

- **HIGH** — the user’s clarified observation, product requirements, existing review sequencing, and independently established imported-only library boundary all agree.

## What Changes for `/10x-plan`

Plan an end-to-end, manually verifiable review experience where a new supported picture shows bounded, advisory similar-picture context from already imported managed images before the user decides. Preserve imported-only similarity scope, explicit Import/Skip controls, local encrypted records, and separate non-previewable skipped-history handling.

## References

- Source files: [`src-tauri/src/review.rs`](../../../src-tauri/src/review.rs), [`src-tauri/src/library.rs`](../../../src-tauri/src/library.rs), [`src-tauri/src/search.rs`](../../../src-tauri/src/search.rs), [`src/app.rs`](../../../src/app.rs)
- Product sources: [`context/foundation/prd.md`](../../foundation/prd.md), [`context/foundation/roadmap.md`](../../foundation/roadmap.md)
- Related research: [`research.md`](research.md)
- Investigation tasks: `/root/frame_visual_signal`, `/root/frame_universe`, `/root/frame_review_timing`

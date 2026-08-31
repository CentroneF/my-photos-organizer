<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Similar-picture context during import review

- **Plan**: context/changes/show-duplicate-and-history-context/plan.md
- **Scope**: Phases 1–3 of 3
- **Date**: 2026-08-31
- **Verdict**: REJECTED
- **Findings**: 1 critical, 5 warnings, 0 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | WARNING |
| Safety & Quality | FAIL |
| Architecture | WARNING |
| Pattern Consistency | PASS |
| Success Criteria | WARNING |

## Verification

- `cargo test --workspace` — PASS: 48 tests passed. The output included a dead-code warning for unused `ReviewState` fields in `src/app.rs`.
- `cargo check --workspace` — PASS: completed successfully with the same warning.
- `cargo tauri build` — INCONCLUSIVE: the command began the release bundle and compiled dependencies, but the available command session ended before a success/failure result was reported.
- Manual criteria — PENDING: the six manual items in `plan.md` were returned to pending because the commits/diff do not contain observable desktop-test evidence.

## Findings

### F1 — Source symlink swap can expose or import an arbitrary file

- **Severity**: ❌ CRITICAL
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/review.rs:325-333, 388-395, 942-951
- **Detail**: Discovery and fingerprinting reject symlinks, but the later preview/import gate uses `fs::metadata`, which follows them. An item can be replaced after fingerprinting with a symlink to an arbitrary readable file. `next_review_item` then adds that path to Tauri's asset scope, and import opens/copies the target. This violates the plan's non-symlink and narrow-source-preview boundary.
- **Fix**: Revalidate sources through a shared `symlink_metadata`-based guard before allowing a preview and immediately before copying; verify recorded metadata and, where feasible, the persisted fingerprint. Add a regression test for replacing a reviewed candidate with an external symlink.
  - Strength: Closes the source-swap class at both external-boundary uses and preserves the existing safe-managed-preview pattern.
  - Tradeoff: Requires threading candidate metadata/fingerprint into the decision-time validation.
  - Confidence: HIGH — the project already uses non-following metadata while discovering/fingerprinting and rejects symlinks for managed previews.
  - Blind spot: A fully race-free path open may require platform-specific descriptor handling beyond the current path-based design.
- **Decision**: SKIPPED

### F2 — Same-size, same-second source replacements can inherit a prior decision

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Plan Adherence
- **Location**: src-tauri/src/review.rs:147-166
- **Detail**: Resume reconciliation treats a revision as unchanged when file size, whole-second `modified_at`, and media type agree. It never compares bytes or a fingerprint. Different bytes with the same size replaced within/preserving the same timestamp second can therefore retain the decided revision instead of queuing a new pending candidate, contrary to the changed-bytes same-path contract. The regression test covers a changed-size replacement only.
- **Fix**: Persist higher-fidelity identity or re-fingerprint a candidate when reconciliation metadata is ambiguous, then append a pending revision whenever the digest differs; add a same-size/same-mtime replacement test.
  - Strength: Makes the revision rule actually byte-aware as the plan requires.
  - Tradeoff: May add bounded I/O when resuming a session.
  - Confidence: HIGH — the BLAKE3 fingerprint machinery already exists in the same module.
  - Blind spot: Filesystem timestamp precision varies by platform, so the exact fast-path policy needs explicit documentation.
- **Decision**: FIXED — re-fingerprint matching-metadata candidates with prior fingerprints and append a revision when bytes differ; regression test added.

### F3 — Similarity threshold is not part of persisted algorithm metadata

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Architecture
- **Location**: src-tauri/src/review.rs:19-24, 688-725; src-tauri/src/library.rs:740, 949
- **Detail**: `SIMILARITY_THRESHOLD` is a Rust constant, while the catalogue stores only the perceptual-hash algorithm/value/state. Changing the constant silently changes the interpretation of existing `dhash-64-v1` values, despite the plan requiring the threshold to be stored alongside the algorithm version so recalibration is a migration/contract change.
- **Fix**: Version the comparison contract (including its threshold) in persisted metadata or encode it in a new algorithm version, with a migration and query filter.
  - Strength: Makes future recalibration explicit and reproducible for existing catalogues.
  - Tradeoff: Adds schema and migration complexity for one fixed threshold.
  - Confidence: HIGH — the catalogue already performs versioned transactional migrations.
  - Blind spot: The best schema shape (per-record vs. catalogue-level contract) has not been selected.
- **Decision**: FIXED — catalogue format 8 persists `perceptual_hash_threshold`; similarity queries require the stored algorithm and threshold to match.

### F4 — Similarity matching materializes and sorts the full imported hash set

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/review.rs:695-725
- **Detail**: The query reads every imported perceptual hash, pushes every within-threshold result into a `Vec`, sorts it, and only then takes three. Returned context is capped, but memory and latency scale with the whole managed library, which conflicts with the plan’s bounded review-context/performance intent.
- **Fix**: Retain only the best three candidates while iterating (for example, a fixed-size ordered collection) and consider a coarse SQL-side filter/index if library scale warrants it.
  - Strength: Enforces the three-item bound in working memory as well as in the DTO.
  - Tradeoff: The calculation still evaluates each stored hash unless the query strategy is also changed.
  - Confidence: HIGH — ordering/tie-break rules are already explicit and can be preserved.
  - Blind spot: No target library-size benchmark is recorded.
- **Decision**: SKIPPED — deferred by user.

### F5 — Phase 3 changes the unrelated S-02 roadmap status

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Scope Discipline
- **Location**: context/foundation/roadmap.md:86-88
- **Detail**: Phase 3 authorized updating S-04 status/notes only. The implementation also changes S-02 to `complete` and attaches the visual-similarity/video-boundary note there. That status may be accurate, but it is unrelated to this change and can make roadmap history misleading.
- **Fix**: Revert the S-02 status/note edits, or document a separately approved reason for them in the appropriate S-02 change.
- **Decision**: FIXED — restored S-02 to `proposed` and removed its unrelated visual-similarity note; S-04 remains complete with its intended note.

### F6 — Manual success criteria are checked without recorded evidence

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: context/changes/show-duplicate-and-history-context/plan.md:230-261
- **Detail**: All manual verification entries were marked complete, including the phase-required human-confirmation pauses, but the reviewed commits only recorded SHA references. Manual desktop-flow and source-preservation evidence could not be established from the diff.
- **Fix**: Record the completed manual scenarios and their result in the change notes or review report; if they were not run, return those progress boxes to pending.
- **Decision**: FIXED — returned all six manual verification items to pending; no manual test evidence was fabricated.

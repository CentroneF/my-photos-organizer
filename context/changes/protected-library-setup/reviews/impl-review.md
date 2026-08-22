<!-- IMPL-REVIEW-REPORT -->
# Implementation Review: Protected library setup

- **Plan**: context/changes/protected-library-setup/plan.md
- **Scope**: Phases 1–2 of 2
- **Date**: 2026-08-22
- **Verdict**: NEEDS ATTENTION
- **Findings**: 0 critical, 6 warnings, 1 observation

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| Plan Adherence | WARNING |
| Scope Discipline | WARNING |
| Safety & Quality | WARNING |
| Architecture | PASS |
| Pattern Consistency | PASS |
| Success Criteria | WARNING |

## Findings

### F1 — Recursive setup cleanup can delete concurrent content

- **Severity**: ⚠️ WARNING
- **Impact**: 🔬 HIGH — architectural stakes; think carefully before deciding
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/library.rs:178
- **Detail**: If setup fails after creating `.photo-handler/`, cleanup recursively removes that path. Another process can add content or replace entries between creation and cleanup, so this is weaker than the plan's requirement to remove only state created by the failed attempt when safe.
- **Fix**: Build state in a uniquely named, exclusively owned temporary directory and atomically rename it into place only after successful initialization; never recursively remove a path that may have changed.
  - Strength: Eliminates the unsafe cleanup window and supports complete-state publication.
  - Tradeoff: Requires restructuring setup and its tests.
  - Confidence: HIGH — standard same-filesystem publication pattern.
  - Blind spot: Platform-specific no-follow APIs still need a macOS/Windows design pass.
- **Decision**: SKIPPED

### F2 — State paths follow symlinks during recovery reset

- **Severity**: ⚠️ WARNING
- **Impact**: 🔬 HIGH — architectural stakes; think carefully before deciding
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/library.rs:324
- **Detail**: Validation and marker replacement follow normal paths below `.photo-handler/`. A symlinked state directory or marker can direct password-reset writes outside the selected library.
- **Fix**: Reject symlinked state components and use directory-handle/no-follow operations when reading or replacing marker files.
  - Strength: Keeps reset mutations contained to the chosen library.
  - Tradeoff: Requires platform-aware filesystem handling.
  - Confidence: HIGH — direct write boundary is identified.
  - Blind spot: Windows reparse-point behavior needs dedicated tests.
- **Decision**: SKIPPED

### F3 — Setup and pointer persistence are not crash-atomic

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/library.rs:156
- **Detail**: Setup writes the database before the marker, and pointer writes truncate in place. A crash can leave an incomplete library that setup will reject as non-empty, or a corrupt pointer; a successful library/password mutation can also be reported as failed if the later pointer write fails.
- **Fix**: Atomically publish complete state and pointer files with temporary files plus rename, and return a distinct result when a library changed but could not be remembered.
  - Strength: Makes restart behavior recoverable without risking existing content.
  - Tradeoff: Adds result states and failure-path tests.
  - Confidence: HIGH — write order and separate mutations are explicit in code.
  - Blind spot: Durable fsync policy remains a product/platform choice.
- **Decision**: FIXED — temporary state/pointer publication with distinct remembered-location messaging

### F4 — Changing libraries retains sensitive form values

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Safety & Quality
- **Location**: src/app.rs:224
- **Detail**: `choose_another` clears only folder and error state. Password, confirmation, recovery answer, and new-password signals remain in UI state and can be reused against the next selected library.
- **Fix**: Add a single sensitive-field clearing helper and call it when switching libraries, on completion, and when cancelling recovery.
- **Decision**: FIXED — credential-related form signals are cleared on switch and completion

### F5 — Post-creation failure cleanup has no direct test

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/library.rs:628
- **Detail**: Tests cover preflight rejection and idempotency, but do not inject a failure after `.photo-handler/` is created. The completed plan criterion for no residual state on invalid initialization is therefore not directly evidenced.
- **Fix**: Make catalogue or marker writing failure-injectable and assert the selected folder has no application state after that late failure.
- **Decision**: SKIPPED

### F6 — Catalogue schema-version rejection lacks a lifecycle test

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Success Criteria
- **Location**: src-tauri/src/library.rs:561
- **Detail**: Code checks `schema_migrations` and `library_identity` versions, but tests only alter the JSON marker version. The plan explicitly calls for migration-version validation.
- **Fix**: Modify a correctly keyed test catalogue to an unsupported schema version and assert unlock rejects it without changing files.
- **Decision**: FIXED — transactional version-0 migration plus newer-version preservation test

### F7 — New privileged commands increase the impact of permissive web settings

- **Severity**: ℹ️ OBSERVATION
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Safety & Quality
- **Location**: src-tauri/src/lib.rs:17
- **Detail**: The feature introduces arbitrary-path filesystem commands while the existing Tauri configuration exposes the global API and disables CSP. Custom commands are not separately capability-gated, so future web-content or navigation changes would increase their risk.
- **Fix**: Treat command exposure as a privileged boundary: restrict CSP/navigation/global API as the desktop shell evolves, and document the trust model.
  - Strength: Reduces the blast radius of any content compromise.
  - Tradeoff: May require frontend/Tauri configuration work beyond this slice.
  - Confidence: MEDIUM — permissive settings predate this feature.
  - Blind spot: Current packaged navigation behavior was not browser-security tested.
- **Decision**: SKIPPED

# Review Media Page UX Changes Implementation Plan

## Overview

Turn the review screen into a full-window decision workspace. It will show the approved Media details fields plus GPS coordinates, persist GPS coordinates with each imported item for future search work, support fast chip-based tagging, and let a reviewer compare every similar imported item in an accessible dialog before choosing to keep both, skip the current item, or safely substitute the old managed copy.

## Current State Analysis

The application is a single Dioxus component whose review state is backed by thin Tauri commands. The review panel uses the viewport height but constrains its inner layout to `42rem`; the existing library workspace uses the available width instead. Review data currently has only basic identity/date/tag fields and advisory exact/similar histories. Similar matches cannot identify their backing imported candidate, are capped at three, and do not affect decisions.

Imports copy source media into the managed library and record a decision. They never mutate the source original. Tags are normalized and associated with a candidate in the same SQLite transaction as its decision, while tag suggestions are imported-only but alphabetical. No existing operation can replace an imported managed copy, transfer its tags, or compensate for a file-system failure after database work.

## Desired End State

The reviewer sees a responsive full-window workspace with Media details—type, size, dimensions, created, modified, captured, camera, and orientation—plus GPS coordinates when present, removable selected tags, and five recent imported tags. When the reviewer imports a copy, its GPS coordinates are saved with that imported decision for future library-search work. Every similar imported match is available for a focused side-by-side comparison. The reviewer can keep both, skip the current candidate, or substitute a selected old managed copy; substitution transfers the normalized union of old and current tags, removes only the old managed copy after a recoverable operation succeeds, and never changes the original source file.

### Key Discoveries:

- The narrow review wrapper is `width: min(100%, 42rem)`, unlike the full-width library wrapper in [assets/styles.css](/Users/fcentron/Desktop/fcentrone/my-photos-organizer/assets/styles.css:29).
- `ReviewItem` and `SimilarMatch` lack the metadata fields and stable imported-candidate identifier the new UI requires ([src-tauri/src/review.rs](/Users/fcentron/Desktop/fcentrone/my-photos-organizer/src-tauri/src/review.rs:36)).
- `import_review_item` copies the source to a managed destination before recording an imported decision, preserving source originals ([src-tauri/src/review.rs](/Users/fcentron/Desktop/fcentrone/my-photos-organizer/src-tauri/src/review.rs:402)).
- `record_decision` owns a SQLite transaction for tags and decisions but cannot atomically commit filesystem changes ([src-tauri/src/review.rs](/Users/fcentron/Desktop/fcentrone/my-photos-organizer/src-tauri/src/review.rs:771)).
- Current tag suggestions are prefix-based and alphabetically ordered, so recent tags require a separate imported-only query ([src-tauri/src/search.rs](/Users/fcentron/Desktop/fcentrone/my-photos-organizer/src-tauri/src/search.rs:214)).

## What We're NOT Doing

- Moving, deleting, or otherwise changing the user's original source media.
- Adding metadata-search controls in this change; the persisted metadata is the foundation for a later search slice.
- Adding comparison for exact matches, changing similarity algorithms/thresholds, or building a duplicate-management screen outside the review flow.
- Creating thumbnails, transcoding video, broadly relaxing asset-protocol permissions, or changing managed-library search behavior other than excluding superseded imports.
- Supporting multi-word tag entry in the review field; per the approved interaction, each space commits one tag.

## Implementation Approach

Deliver three vertical, frontend-verifiable slices. The first improves every review decision with full-window information and tags. The second makes all existing visual-similarity results actionable through a single accessible comparison dialog. The third introduces the schema relationship and compensating file/database workflow needed for safe substitution.

The metadata DTO will contain the approved filesystem/header/EXIF fields—type, size, dimensions, created, modified, captured, camera, and orientation—plus decimal GPS latitude and longitude when available. `import_review_item` persists only the GPS coordinate pair with the imported decision. Unsupported/corrupt formats retain available Media details, report GPS as unavailable, and never block review, comparison, import, or skip. No metadata-search filters or indexes are added in this change.

## Critical Implementation Details

Substitution spans filesystem and SQLite, so it cannot be globally atomic. Publish the incoming managed copy first, create a checked recovery hard-link for the old managed copy, perform the replacement database transaction, then remove the old managed copy before committing. If cleanup or commit fails, roll back/compensate, leave the current candidate undecided, preserve recoverable copies, and return an actionable error; source originals are never touched.

## Phase 1: Full-Window Review Information and Tags

### Overview

Make the normal review experience use the full application window and provide all approved safe metadata plus fast, visible tag selection. This slice remains usable for import and skip even when metadata cannot be extracted.

### Changes Required:

#### 1. Review metadata and GPS import-persistence contract

**File**: `src-tauri/Cargo.toml`

**Intent**: Add the local metadata parser required to read the approved Media details and GPS coordinates without granting the frontend filesystem access.

**Contract**: Add the selected EXIF dependency and update `Cargo.lock`. The parser remains native-only and reads source files solely through the existing safe review path.

**File**: `src-tauri/src/review.rs`

**Intent**: Project the approved filesystem/header/EXIF details and GPS coordinates into the active review item, and preserve GPS coordinates when the candidate is imported.

**Contract**: Add serializable Media details for type, size, dimensions, created, modified, captured, camera, and orientation, plus an optional decimal GPS latitude/longitude pair. Metadata probes run only after source stability checks (or revalidate afterward); unavailable probes become labelled unavailable values and never produce a review error. Extend `ImportRequest`/`import_review_item` to persist the GPS pair atomically with a successful imported decision; skipped candidates do not receive GPS data.

**File**: `src-tauri/src/library.rs`

**Intent**: Persist immutable GPS coordinates for imported media without forcing a schema per EXIF vendor key.

**Contract**: Bump the encrypted catalogue format and migrate `item_decisions` with a nullable serialized GPS payload column. New imports write the payload in the same transaction as their imported decision; existing imports retain `NULL` until a future backfill decision. Do not add metadata-search indexes or alter existing search controls.

**File**: `src/app.rs`

**Intent**: Display the approved Media details and GPS coordinates for the current review item in an understandable full-window section.

**Contract**: Render Type, Size, Dimensions, Created, Modified, Captured, Camera, Orientation, and GPS coordinates with stable labels/values and clear unavailable states. Retain existing filename/path/date information, preview fallback, Import, and Skip actions.

#### 2. Full-window layout and chip-based review tags

**File**: `src-tauri/src/search.rs`

**Intent**: Supply the five most recently used imported-library tags for review shortcuts.

**Contract**: Add an authenticated native query/result returning at most five unique normalized tags, ordered by each tag's latest imported `item_decisions.decided_at` descending with deterministic tie-breaking. Skipped-only tags are excluded.

**File**: `src-tauri/src/lib.rs`

**Intent**: Register the recent-tags command through the existing Tauri command boundary.

**Contract**: The public command name and `src/app.rs` invocation string stay synchronized and use the existing structured error behavior.

**File**: `src/app.rs`

**Intent**: Replace the comma-separated review-tags field with space-committed removable chips and add recent-tag toggles.

**Contract**: Maintain selected tags as `Vec<String>` plus a draft value. Spaces (including pasted whitespace) commit normalized, nonempty, deduplicated tokens; each selected chip has an accessible `×` remove button; recent-tag buttons toggle the same selection with `aria-pressed`. Reset selected tags/draft from the next review item, and submit the selected vector to Import and Skip rather than splitting a string.

**File**: `assets/styles.css`

**Intent**: Expand only the review workspace to the available window and style the Media details/GPS section, chips, recent toggles, and narrow-screen reflow.

**Contract**: Remove the review-only narrow width cap while leaving other flow wrappers unaffected. The review content remains scrollable and decision actions remain available at viewport-constrained heights.

#### 3. Phase-one automated coverage

**File**: `src-tauri/src/review.rs`

**Intent**: Prove approved Media details/GPS extraction and GPS persistence for supported EXIF, missing, unsupported, and malformed metadata sources.

**Contract**: Tests cover the eight displayed Media details, decimal GPS extraction, non-blocking unavailable metadata, and persistence/reopen of imported GPS data. Confirm the source remains unmodified.

**File**: `src-tauri/src/search.rs`

**Intent**: Prove recent-tag recency, imported-only filtering, unique results, limit five, and deterministic ordering.

**Contract**: Use the existing protected-catalogue test setup and exercise actual decision timestamps and tag associations.

**File**: `src/app.rs`

**Intent**: Guard the new review layout/metadata/chip contracts in the project's existing source-and-CSS test style.

**Contract**: Assert the full-window selector, Media details/GPS presentation, GPS unavailable state, space-commit path, accessible remove controls, recent-tag toggle hooks, and vector-based decision requests.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including Media details/GPS extraction and GPS-persistence migration/reopen, recent-tag ordering, and review UI source/CSS contract coverage.
- `cargo tauri build` succeeds with the metadata dependency and recent-tags command registered.

#### Manual Verification:

- Review a supported GPS-tagged image and an unsupported/corrupt-metadata media file; both show a full-window review workspace with all approved Media details and a labelled unavailable GPS state without blocking actions.
- Import the GPS-tagged image, reopen the protected library catalogue, and verify its GPS coordinates remain persisted for a future metadata-search feature.
- Enter and paste space-separated tags, remove chips with `×`, toggle recent imported tags, then Import and Skip items to verify the selected tags persist as expected.
- Resize a review session to a narrow/tall window and verify details can scroll while Import and Skip remain reachable.

**Implementation Note**: After the automated checks pass, pause for human confirmation of this review workflow before starting Phase 2.

---

## Phase 2: Compare Every Similar Imported Item

### Overview

Replace bounded advisory similar-history rows with a complete, deterministic list of imported visual matches and let each row open a side-by-side dialog. The dialog supports a safe decision to keep both or skip the current candidate while retaining the normal review queue behavior.

### Changes Required:

#### 1. Full similar-match native contract

**File**: `src-tauri/src/review.rs`

**Intent**: Return every qualifying, non-superseded similar imported candidate with the stable identity required for comparison actions.

**Contract**: Extend `SimilarMatch` with its imported `candidate_id`, remove the current presentation limit, and order results newest imported decision first with a deterministic secondary key. Retain managed-only safe preview URLs and never expose skipped/source-only candidates.

**File**: `src/app.rs`

**Intent**: Deserialize the stable match identifier and turn each similar row into a comparison trigger.

**Contract**: Similar rows remain scrollable and include a clear Compare control without auto-opening a dialog or hiding normal review actions.

#### 2. Accessible side-by-side comparison dialog and decisions

**File**: `src/app.rs`

**Intent**: Add a review-local selected-match state and focused dialog for comparing the current candidate to one imported match.

**Contract**: Render a sibling overlay with `role="dialog"`, `aria-modal="true"`, an accessible title, equal current/imported panels, and labelled preview fallbacks. Escape and a visible Close/Cancel control dismiss it; backdrop interaction does not. Initial focus is placed within the dialog. Keep Both routes through the ordinary import decision; Skip routes through the ordinary skip decision. Both retain existing busy/error/next-item sequencing, and an error leaves the dialog/match selection open.

**File**: `assets/styles.css`

**Intent**: Add the dialog overlay/panel and responsive comparison presentation.

**Contract**: The panel has bounded viewport height with internal scrolling; comparison panels are side-by-side at regular widths and stack at narrow widths; keyboard focus is visibly styled.

#### 3. Phase-two automated coverage

**File**: `src-tauri/src/review.rs`

**Intent**: Verify all valid matches are returned in the approved order with server-authoritative identifiers.

**Contract**: Tests cover no artificial three-match limit, imported-only filtering, deterministic newest-first ordering, and safe preview absence where unavailable.

**File**: `src/app.rs`

**Intent**: Preserve dialog and decision behavior in the existing frontend contract-test style.

**Contract**: Assert Compare triggers, dialog ARIA attributes, Escape/Close behavior, absence of backdrop dismissal, busy/error retention, side-by-side responsive styles, and the existing Keep Both/Skip command pathways.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with all-match ordering and comparison-dialog source/CSS contract coverage.
- `cargo tauri build` succeeds with the expanded review response contract.

#### Manual Verification:

- Review a candidate with more than three similar imported pictures and confirm every match is reachable, ordered newest-first, and opens its own comparison.
- Use Escape and Close/Cancel to leave the dialog without deciding; verify backdrop clicks do not close it and keyboard focus remains usable.
- From comparison, choose Keep Both and Skip; confirm the expected decision is recorded, the next review item appears, and the original source file remains unchanged.

**Implementation Note**: After the automated checks pass, pause for human confirmation of the comparison workflow before starting Phase 3.

---

## Phase 3: Safe Substitute and Tag Transfer

### Overview

Add a dedicated substitute decision that replaces one selected old managed import with the current candidate only after a compensating file/database operation can preserve recovery. The tag result is the normalized union of the old imported tags and current selected tags.

### Changes Required:

#### 1. Replacement persistence and visibility rules

**File**: `src-tauri/src/library.rs`

**Intent**: Persist a replacement relationship while retaining historical decisions for audit/recovery.

**Contract**: Bump the catalogue format and migrate `item_decisions` with a nullable replacement reference to the incoming candidate plus query support. Existing records remain valid. Superseded imported decisions are excluded from library search, future similar-match results, and active history presentation as appropriate; their data is not deleted from the catalogue.

**File**: `src-tauri/src/search.rs`

**Intent**: Keep superseded managed imports out of normal library results.

**Contract**: Imported-library queries explicitly exclude decisions marked as replaced while retaining the existing imported-only and preview-containment boundary.

#### 2. Dedicated compensating substitute operation

**File**: `src-tauri/src/review.rs`

**Intent**: Implement a server-authoritative substitute command for the current pending candidate and selected imported match.

**Contract**: Accept the current candidate ID, selected imported candidate ID, selected tags, and effective import date. Validate both candidates, source stability, target import state, unreplaced state, and safe managed destination before any mutation. Publish the incoming copy, create a checked recovery hard-link for the old managed copy, calculate the normalized union of old/current tags, write the incoming imported decision and old replacement relationship in a transaction, remove the old managed copy, then commit. On any publish, backup, remove, or commit failure, leave the current candidate undecided, preserve/restore recoverable copies where possible, avoid source mutation, and return an actionable structured error.

**File**: `src-tauri/src/lib.rs`

**Intent**: Register the substitute command at the native boundary.

**Contract**: Its frontend invocation name matches the registered command; it requires no broadened capabilities or direct filesystem access from the UI.

**File**: `src/app.rs`

**Intent**: Expose Substitute only within the selected similar-match dialog and refresh review state after a successful command.

**Contract**: The request carries only current/selected candidate IDs, selected tags, and effective date. Disable dialog outcomes while busy; on success clear dialog state and advance exactly once; on error retain dialog context and show the actionable native message.

#### 3. Substitute-focused safety coverage

**File**: `src-tauri/src/review.rs`

**Intent**: Exercise normal and fault-injected substitute outcomes against a temporary protected library.

**Contract**: Cover success; normalized tag union; source-byte preservation; rejection of same/pending/skipped/already-replaced/outside-library/symlink/missing targets; copy failure; recovery-link failure; old-copy removal failure; and commit failure/restore behavior. Verify failed substitutions leave no current decision and preserve discoverable recovery details.

**File**: `src-tauri/src/library.rs` and `src-tauri/src/search.rs`

**Intent**: Verify migration/reopen persistence and exclusion of superseded imports.

**Contract**: Tests confirm that reopen preserves the relationship, the old managed record is absent from normal search/similar matching, and the new imported record remains visible with merged tags.

**File**: `src/app.rs`

**Intent**: Guard Substitute request/response and dialog error-retention behavior.

**Contract**: Source-level tests assert that Substitute is dialog-scoped, submits the selected IDs/tags/date, has busy protection, and does not advance/close after an error.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with migration, successful substitute, tag-union, rejection, and fault-recovery coverage.
- `cargo tauri build` succeeds with the substitute command and review dialog integration.

#### Manual Verification:

- Compare a current image to an imported match, choose Substitute, and verify the new managed copy appears in library search with the merged tags while the old managed copy is absent.
- Confirm the candidate's original source file remains in its import folder with unchanged bytes after a successful substitute.
- Simulate or induce a safe cleanup failure in a development library and verify the dialog reports an actionable error, the review candidate remains pending, and recoverable copies remain available.

**Implementation Note**: After automated verification passes, pause for human confirmation of successful substitute and recovery behavior before declaring the change complete.

## Testing Strategy

### Unit Tests:

- The approved Media details and decimal GPS coordinates are non-blocking, and GPS persists only with imported decisions.
- Recent imported tags use per-tag latest imported decision, deterministic ordering, and a five-tag cap.
- Similar matching returns each valid non-superseded imported candidate in newest-first order.
- Substitute validates identity/state/path invariants, preserves source bytes, normalizes tag unions, and compensates for filesystem/database failures.

### Integration Tests:

- Use temporary encrypted libraries to verify GPS persistence/reopen, recent-tag results, all-match comparison contracts, substitute migrations, and search visibility.
- Inject filesystem/transaction seams to verify failure recovery before a real managed destination can be removed.

### Manual Testing Steps:

1. Start a review with supported GPS-tagged, unsupported, and metadata-poor media; verify the full-window Media details/GPS workspace and non-blocking fallback labels.
2. Use space-delimited tag entry, remove chips, and select recent tags before both import and skip decisions.
3. Compare every similar imported item through the keyboard-accessible dialog; exercise close, Keep Both, and Skip behavior.
4. Substitute a selected imported match and confirm merged tags, updated library visibility, original-source preservation, and recovery messaging for a deliberately failed cleanup.

## Performance Considerations

Metadata extraction is one safe per-candidate probe, with image dimensions read from headers rather than decoded pixels. Similar-match results are intentionally unbounded to meet the requirement, so the database query must remain indexed and the UI list/dialog panel must scroll rather than eagerly render full-size media. Recent tags are a five-row aggregate query. No thumbnails, full-library EXIF scans, or source-folder traversal are introduced.

## Migration Notes

The catalogue format is incremented for the replacement relationship and supporting index/migration. Existing imported decisions remain intact and initially unreplaced; no historical managed files are scanned or deleted during migration. A replaced decision remains catalogue history but is omitted from active library search and subsequent similarity matching. The failure path may intentionally retain a new managed copy and/or recovery hard link until the user can retry or recover; it must never remove the user’s source original.

## References

- Requirements: `context/changes/review-media-page-ux-changes/requirments.md`
- Vertical-slice rule: `context/foundation/lessons.md`
- Current review response, matching, and decisions: `src-tauri/src/review.rs:36-73`, `src-tauri/src/review.rs:248-428`, `src-tauri/src/review.rs:702-800`
- Current command registration: `src-tauri/src/lib.rs:6-35`, `src-tauri/src/lib.rs:237-257`
- Current review UI and test conventions: `src/app.rs:659-784`, `src/app.rs:1194-1365`
- Layout and library tag suggestions: `assets/styles.css:27-113`, `src-tauri/src/search.rs:214-231`
- Existing advisory duplicate context: `context/changes/show-duplicate-and-history-context/research.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Full-Window Review Information and Tags

#### Automated

- [ ] 1.1 Add Media details/GPS extraction, GPS persistence, response contract, and metadata display
- [x] 1.2 Add recent imported-tag query and register its Tauri command
- [x] 1.3 Implement the full-window review layout and space-delimited removable tag chips
- [ ] 1.4 Run `cargo test --workspace` for metadata, recent-tag, and UI contract coverage
- [ ] 1.5 Run `cargo tauri build`

#### Manual

- [ ] 1.6 Verify full-window metadata fallbacks, chip tagging, recent toggles, and responsive review actions

### Phase 2: Compare Every Similar Imported Item

#### Automated

- [ ] 2.1 Extend similar-match results with stable IDs and all-match newest-first ordering
- [ ] 2.2 Implement the accessible side-by-side comparison dialog with Keep Both and Skip actions
- [ ] 2.3 Run `cargo test --workspace` for matching and dialog contract coverage
- [ ] 2.4 Run `cargo tauri build`

#### Manual

- [ ] 2.5 Verify all-match access, keyboard dismissal, retained errors, Keep Both, and Skip behavior

### Phase 3: Safe Substitute and Tag Transfer

#### Automated

- [ ] 3.1 Add the replacement migration and exclude superseded imports from active queries
- [ ] 3.2 Implement the compensating substitute operation and register its Tauri command
- [ ] 3.3 Integrate Substitute into the comparison dialog with busy/error handling
- [ ] 3.4 Run `cargo test --workspace` for migration, success, rejection, and recovery paths
- [ ] 3.5 Run `cargo tauri build`

#### Manual

- [ ] 3.6 Verify substitute tag transfer, source preservation, library visibility, and recoverable cleanup failure handling

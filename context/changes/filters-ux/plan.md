# Filters UX Implementation Plan

## Overview

Rework the Library Search controls into a more focused workspace: filters live in an independently expandable panel on the left, active constraints are visible and removable above the media grid, and library actions move into one top-right settings menu. Extend the existing imported-only encrypted-catalogue search contract only where the requested behaviors require it: two simultaneous date ranges, checkbox media-type selection, and frequency-ranked tag discovery.

## Current State Analysis

Library Search is the `home` branch of the single Dioxus application component. Its header contains the requested-to-remove `LIBRARY SEARCH` label, explanatory lede, and four always-visible action buttons. Its filters are a top-of-grid three-column form: one selected/original date-mode selector plus one range, one media-type select, and tag prefix suggestions.

`search_library` already queries only active managed imports through the authenticated `library::with_catalogue` boundary. It accepts one date range and one optional media type; selected tags are normalized and combined with AND semantics. Safe preview URL validation is already deliberately separate from the UI and must remain unchanged.

## Desired End State

An unlocked library shows a clean media workspace with a top-right settings icon menu and a left-side filter panel. All filter sections can remain open. Imported and Captured ranges combine as AND predicates; Images and Videos start selected and can be toggled individually; an empty type selection shows no results. Each active constraint has a removable applied-filter chip above the grid, with a Clear all action.

The Tags section initially shows the ten most-used tags from active imported media. A tag search field performs literal, case-normalized substring matching, returns at most ten frequency-ranked tags, and each tag toggles into the same AND-based search state. All behavior remains local, imported-only, and immediately updates the grid.

### Key Discoveries:

- Existing search UI state and Tauri request serialization are colocated in `src/app.rs:313-347` and `src/app.rs:491-640`.
- The current query’s `decision = 'imported'`, managed-destination, and non-replaced predicates define the product’s safe search population at `src-tauri/src/search.rs:168-181`.
- Both `effective_import_date` and nullable `original_media_date` already exist; independent ranges need query-contract changes, not a catalogue migration.
- Existing tag normalization and `escape_like` make a prepared, literal substring query possible at `src-tauri/src/search.rs:262-281`.
- The header, filter form, and grid have a compact responsive style surface at `assets/styles.css:107-135`.

## What We're NOT Doing

- Searching source folders, skipped decisions, superseded imports, filenames, or external/cloud media.
- Changing the managed-preview containment checks, filesystem permissions, or the rule that original media is never moved or deleted.
- Adding saved filters, pagination, tag editing, tag counts in the UI, an AND/OR switch, or a new routing/state-management framework.
- Backfilling captured dates for legacy imports; records with a null captured date simply do not match a captured-date constraint.

## Implementation Approach

Deliver four small, vertical slices. First relocate the existing experience into the new workspace and make its currently active filters visible/removable. Then independently replace date semantics, media selection, and tag discovery end-to-end, updating the native DTO/query, registered command where needed, Dioxus state/markup, styles, and coverage in the same phase.

The search request remains reactive: every committed filter change immediately invokes the local Tauri command. The native query continues to own normalization, date validation, imported-only selection, and deterministic ordering; UI state never grants filesystem access.

## Critical Implementation Details

Keep the existing safe-preview boundary intact: a catalogue destination is not itself authority to expose a file. `safe_preview_url` must continue to require a readable, regular, non-symlink managed file beneath the canonical library root before it calls `allow_file`.

For tag substring matching, bind `format!("%{}%", escape_like(&normalize_tag(query)))` to a prepared `LIKE ? ESCAPE '\\'` clause. The wrapper wildcards provide substring behavior; escaped user `%`, `_`, and backslash characters remain literal.

## Phase 1: Workspace shell and existing-filter visibility

### Overview

Move the current Library Search controls into a left-side multi-expand panel and make current constraints visible and removable above the grid, while preserving the existing native search contract and all library action routes.

### Changes Required:

#### 1. Library Search workspace and interaction state

**File**: `src/app.rs`

**Intent**: Replace the current four-button header and top filter form in the `home` branch with a title plus accessible top-right settings-menu trigger, a menu containing Import media, Library settings, Close library, and Danger zone, a left-side filter panel, and an applied-filter bar above the results.

**Contract**: The menu actions use the current `step` transitions and `close_library` handler; no new native command is introduced. Each filter section has its own expanded signal and may stay open with other sections. The moved date-mode/range, media select, and current tag controls preserve their existing request semantics in this phase. Applied chips remove only their represented state; Clear all restores the existing default state and triggers the current reactive search.

#### 2. Responsive workspace styling

**File**: `assets/styles.css`

**Intent**: Replace the top-form layout with a grid/flex workspace that places the result column beside the left filter sidebar on wide screens and stacks it cleanly on narrow screens; add menu, disclosure, filter-chip, and focus-visible styles.

**Contract**: The sidebar sections are keyboard-operable and visually indicate expanded state. The action menu remains associated with its trigger, closes without blocking navigation, and all existing empty-state, grid, and preview fallback layouts remain usable at the existing 820px and 520px breakpoints.

#### 3. Frontend contract coverage

**File**: `src/app.rs`

**Intent**: Update the lightweight source/CSS tests to guard the workspace, action-menu, independently expandable filters, applied-filter controls, and removal of the two specified header texts.

**Contract**: Tests assert stable user-facing hooks/labels rather than browser-layout behavior; native search semantics remain covered by existing native tests until later phases change them.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with workspace/action-menu/applied-filter source and CSS contract coverage.
- `cargo check --workspace` succeeds without producing a DMG bundle.

#### Manual Verification:

- Unlock a populated library and verify filters appear on the left, multiple sections remain expanded, and the existing date/type/tag controls still update the grid.
- Open the top-right settings menu and verify every existing action reaches the same Import media, Library settings, Close library, and Danger zone flows.
- Apply and individually remove existing filters, use Clear all, and confirm neither `LIBRARY SEARCH` nor “Browse imported copies. Originals remain untouched.” is shown.

**Implementation Note**: After automated verification passes, pause for human confirmation of the manual checks before Phase 2.

---

## Phase 2: Independent imported and captured date ranges

### Overview

Replace the mutually exclusive date-mode selector with two independent inclusive ranges that can be combined, and expose each active bound as an applied-filter chip.

### Changes Required:

#### 1. Native dual-range search contract and query

**File**: `src-tauri/src/search.rs`

**Intent**: Replace the single `date_field`/range request representation with optional imported and captured start/end fields, validate both ranges independently, and add every supplied bound to the imported-media query.

**Contract**: A request with filled Imported and Captured constraints uses AND semantics. Each start must be on or before its corresponding end; invalid ISO dates and inverted ranges return the existing structured search error shape. A captured constraint naturally excludes legacy rows whose `original_media_date` is null. Retain imported-only/non-replaced predicates, tag AND semantics, preview behavior, and deterministic result ordering by effective import date then candidate ID. No database migration is required.

#### 2. Registered command and UI request synchronization

**Files**: `src-tauri/src/lib.rs`, `src/app.rs`

**Intent**: Keep the public `search_library` command and frontend invocation synchronized while updating Dioxus signals, request serialization, filter disclosure content, legacy-date explanation, and applied-chip rendering for both date ranges.

**Contract**: The UI has four date inputs: Imported from/to and Captured from/to. Leaving any individual bound blank means it is unconstrained. Changing or removing one bound immediately refreshes the grid without clearing the other range; Clear all resets both ranges. Errors from either range appear through the existing page error mechanism.

#### 3. Native and frontend regression coverage

**Files**: `src-tauri/src/search.rs`, `src/app.rs`

**Intent**: Extend the encrypted-catalogue fixture and UI source-level tests for dual-range behavior.

**Contract**: Native tests cover each range independently, their intersection, invalid dates, inverted ranges, and null captured dates. UI tests assert the four controls, independent request fields, applied-chip hooks, and the retained legacy-record explanation.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including independent date validation and combined-range imported-only query coverage.
- `cargo check --workspace` succeeds with the updated Tauri/Dioxus request contract and without producing a DMG bundle.

#### Manual Verification:

- In a library with different imported and captured dates, verify either range filters independently and both together show only their intersection.
- Remove one date chip and confirm only that bound clears; Clear all restores the unfiltered grid.
- Verify an older imported record with no captured date remains searchable by imported date but does not match a captured-date filter.

**Implementation Note**: After automated verification passes, pause for human confirmation of the manual checks before Phase 3.

---

## Phase 3: Checkbox media-type filtering

### Overview

Replace the single media-type selector with explicit Images and Videos checkboxes, preserving instant search and making their current state visible above the grid.

### Changes Required:

#### 1. Multi-type native search contract and predicate

**File**: `src-tauri/src/search.rs`

**Intent**: Change the singular optional media type to a normalized, duplicate-free collection and filter matching imported records by any selected type.

**Contract**: The request accepts the supported `image` and `video` values only. The default request selects both; an empty collection is an explicit no-results condition, rather than silently removing the media constraint. One or both selected types are applied with a parameterized OR/`IN` predicate. No schema migration is required.

#### 2. Checkbox UI and applied-filter integration

**File**: `src/app.rs`

**Intent**: Replace the media select in its sidebar disclosure with two accessible checkboxes and update the reactive request and applied-filter bar.

**Contract**: Images and Videos initialize selected. Toggling a checkbox immediately refreshes the grid; the applied bar reports the selected type(s), supports individual removal, and makes a deliberately empty selection understandable through the normal no-results state. Clear all restores both selections.

#### 3. Visual and contract coverage

**Files**: `assets/styles.css`, `src-tauri/src/search.rs`, `src/app.rs`

**Intent**: Style selectable media-type controls consistently with tag chips/disclosures and cover collection semantics at both layers.

**Contract**: Native fixture tests cover image-only, video-only, both-type, duplicate-value normalization, and empty-selection behavior. Frontend tests assert checkbox labels/state hooks and applied-filter controls; responsive styling keeps the controls reachable in the stacked layout.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including multi-type query semantics and checkbox source/CSS coverage.
- `cargo check --workspace` succeeds with the synchronized multi-type search DTO and without producing a DMG bundle.

#### Manual Verification:

- Toggle Images and Videos independently in a populated library and confirm only matching cards remain.
- Clear both checkboxes and confirm the UI communicates an intentional no-results state; use Clear all and confirm both types return.
- Remove a media-type chip and confirm only that type’s selection changes while date and tag filters remain intact.

**Implementation Note**: After automated verification passes, pause for human confirmation of the manual checks before Phase 4.

---

## Phase 4: Frequency-ranked tag discovery

### Overview

Replace prefix suggestions with an imported-only tag browser that starts with the ten most-used tags and narrows through literal substring search, while retaining selected-tag AND semantics.

### Changes Required:

#### 1. Tag-list command and authenticated catalogue query

**Files**: `src-tauri/src/search.rs`, `src-tauri/src/lib.rs`

**Intent**: Replace the Library Search suggestion endpoint with an authenticated tag-list endpoint and register it through the existing Tauri invoke handler.

**Contract**: `list_library_tags` accepts an optional query and returns at most ten normalized tag names. Blank/whitespace input returns the top ten active imported tags by frequency; nonblank input applies a prepared literal substring `LIKE` match, then ranks by frequency descending and normalized name ascending. The query counts only active imported managed rows, excludes skipped/replaced/unmanaged tags, and uses the current normalization and `escape_like` helpers. `recent_library_tags` remains unchanged for the review screen.

#### 2. Searchable tag-panel UI

**File**: `src/app.rs`

**Intent**: Replace the typeahead field/suggestions with a Tags-panel search field and toggleable tag list that refreshes when the panel opens or its query changes.

**Contract**: A blank query presents the top ten; a one-or-more-character query requests literal substring results. Tag buttons add/remove the normalized tag from `search_selected_tags`; selected tags are visually pressed and appear as individually removable applied-filter chips. Multiple selected tags continue to use the existing AND behavior, and filters update the grid immediately.

#### 3. Tag-panel presentation and regression coverage

**Files**: `assets/styles.css`, `src-tauri/src/search.rs`, `src/app.rs`

**Intent**: Provide a bounded, scrollable tag result area inside the expandable sidebar and prove ranking, safety, substring semantics, and UI hooks.

**Contract**: Native fixture tests cover blank top-ten frequency ranking, alphabetical tie breaks, normalized mixed-case/whitespace input, literal `%`/`_` escaping, substring rather than prefix matching, and exclusion of skipped/replaced-only tags. Frontend tests assert the new command, tag search field, toggle state, and removal of the obsolete suggestion-only hooks.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including frequency ranking, literal substring escaping, imported-only tag discovery, and selected-tag AND filtering.
- `cargo check --workspace` succeeds with the registered tag-list command and Dioxus tag panel, without producing a DMG bundle.

#### Manual Verification:

- Open Tags in a populated library and confirm the initial list shows at most ten common imported tags; uncommon/skipped-only tags are absent.
- Search for text occurring in the middle of a tag, select and re-select tags, and verify selected states and applied chips track the grid immediately.
- Combine tags with dates and media types and verify only imported media carrying every selected tag and matching all other active filters remains.

**Implementation Note**: After automated verification passes, pause here for human confirmation of the manual checks before declaring the change complete.

## Testing Strategy

### Unit Tests:

- Validate both inclusive date ranges independently and together, including malformed dates, inverted ranges, and null captured dates.
- Verify media-type collection normalization, image/video matching, and explicit empty-selection behavior.
- Verify tag frequency ranking, deterministic ties, literal substring escaping, imported-only discovery, and selected-tag AND filtering.
- Retain existing managed-preview containment and imported-only query coverage.

### Integration Tests:

- Use the existing temporary protected-library fixture to issue native search/tag-list requests against encrypted catalogue data containing imported, skipped, replaced, legacy-date, image, and video rows.
- Run `cargo test --workspace` and `cargo check --workspace` after each phase; do not build a DMG.

### Manual Testing Steps:

1. Unlock a populated library and verify the top-right action menu, left filter panel, and responsive stacked layout.
2. Apply, remove, and clear imported/captured ranges; verify expected intersection and legacy captured-date behavior.
3. Toggle Images/Videos and confirm the explicit empty-selection state, then restore defaults.
4. Browse and substring-search the top-ten tag list; combine selected tags with the other filters and verify only active imported media remains.

## Performance Considerations

All queries remain local to the encrypted catalogue. The tag list is capped at ten results, and the tag search uses a parameterized `LIKE '%term%'`, which may not use a prefix index; that cost is intentionally bounded by the response limit and local catalogue scope. Do not add a migration, FTS index, or thumbnail work unless profiling demonstrates a real issue.

## Migration Notes

No catalogue migration is expected: imported/captured dates and media types already exist, and the work only changes request/query and presentation contracts. Legacy rows with null captured dates retain their existing behavior. No capability permissions change.

## References

- Requirements: `context/changes/filters-ux/requirments.md`
- Existing Library Search plan: `context/changes/search-managed-library/plan.md`
- UI request/state/effects: `src/app.rs:313-347`, `src/app.rs:491-640`
- Current Library Search markup: `src/app.rs:1333-1352`
- Native search/query/tag helpers: `src-tauri/src/search.rs:11-281`
- Tauri command registration: `src-tauri/src/lib.rs:57-75`, `src-tauri/src/lib.rs:262-286`
- Current search styles: `assets/styles.css:107-135`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Workspace shell and existing-filter visibility

#### Automated

- [x] 1.1 Build the left-side expandable filter workspace, applied-filter bar, and top-right action menu while preserving existing filters
- [x] 1.2 Add responsive workspace/menu/filter styling and frontend source/CSS contract coverage
- [x] 1.3 Run `cargo test --workspace` and `cargo check --workspace`

#### Manual

- [x] 1.4 Verify the workspace, existing filters, action routes, applied-filter removal, and requested header-text removal

### Phase 2: Independent imported and captured date ranges

#### Automated

- [ ] 2.1 Add the dual-range native search request, validation, AND predicates, and encrypted-catalogue coverage
- [ ] 2.2 Wire four date controls and their applied-filter chips to the synchronized search command
- [ ] 2.3 Run `cargo test --workspace` and `cargo check --workspace`

#### Manual

- [ ] 2.4 Verify independent and combined date ranges, chip removal, and legacy captured-date behavior

### Phase 3: Checkbox media-type filtering

#### Automated

- [ ] 3.1 Add multi-type native search semantics and encrypted-catalogue coverage
- [ ] 3.2 Replace the media selector with checkbox controls, applied-filter behavior, responsive styles, and source coverage
- [ ] 3.3 Run `cargo test --workspace` and `cargo check --workspace`

#### Manual

- [ ] 3.4 Verify image/video toggles, explicit no-results behavior, Clear all, and chip removal

### Phase 4: Frequency-ranked tag discovery

#### Automated

- [ ] 4.1 Add and register the imported-only frequency-ranked tag-list command with literal substring coverage
- [ ] 4.2 Replace suggestion UI with searchable, toggleable top-ten tag controls and applied-filter integration
- [ ] 4.3 Run `cargo test --workspace` and `cargo check --workspace`

#### Manual

- [ ] 4.4 Verify top-ten ranking, substring discovery, tag toggling, and combined-filter AND behavior

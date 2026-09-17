# Open street map view Implementation Plan

## Overview

Add a map mode to the Library filters workspace. The user can replace the current media grid with an interactive OpenStreetMap view of the currently filtered, imported images that have valid saved GPS coordinates. Nearby photographs cluster for readability; photos at an identical coordinate fan out as selectable rays, and selecting one opens the existing managed-media preview.

## Current State Analysis

The Library's `home` branch already owns reactive date, media-type, and tag filters, and sends their combined request to the authenticated `search_library` Tauri command. Its result records intentionally contain only card data, so the map cannot currently use GPS without an N+1 preview-details request.

GPS is parsed during review/import and persisted as nullable `item_decisions.gps_json`; it is therefore available locally without reading the original or re-reading the managed copy. There is no map renderer, map asset, or map-specific frontend state today. The existing media preview overlay offers the established full-window inspection and close behavior that map pin selection can reuse.

## Desired End State

An unlocked Library has an accessible `Show map` control. Activating it replaces the result list with a map that contains only matching imported **images** with valid stored GPS data; videos, missing coordinates, legacy nulls, malformed coordinates, replaced imports, and unmanaged files never produce markers.

The map initially fits all current markers, clusters nearby locations, and expands a maximum-zoom cluster into visible rays for same-coordinate selections. Clicking a single pin opens the existing preview; closing that preview returns to the map. Back-to-list restores the same active filters and the prior grid scroll position. An empty GPS result has a useful empty map state, while failed tile loading explains that internet access is required and supplies the same recovery path. Visible OpenStreetMap attribution remains on the map.

### Key Discoveries:

- The reactive library request and result replacement live in `src/app.rs:696-755`; current filters are assembled into `SearchLibraryRequest` at `src/app.rs:700-719`.
- The current map-relevant UI is the `home` Library workspace at `src/app.rs:1792-1904`; the grid and filters already share a responsive layout.
- `SearchLibraryItem` has no GPS field at either the frontend DTO (`src/app.rs:305-321`) or native DTO (`src-tauri/src/search.rs:60-76`), while `load_preview` accepts a result index and can be reused at `src/app.rs:1399-1433`.
- GPS is a nullable JSON column introduced at `src-tauri/src/library.rs:813` and repaired for legacy catalogues at `src-tauri/src/library.rs:1069-1090`; no migration is needed to read it.
- OSM's standard tile service requires HTTPS, visible attribution, normal interactive viewport loading, and no prefetch/offline tile download: [Tile Usage Policy](https://operations.osmfoundation.org/policies/tiles/).
- Leaflet's map API supports bounds fitting and accessible map interaction; Leaflet.markercluster supports nearby clustering and max-zoom spiderfying of overlapping markers: [Leaflet reference](https://leafletjs.com/reference.html), [markercluster documentation](https://github.com/Leaflet/Leaflet.markercluster).

## What We're NOT Doing

- Reading GPS from source media or managed files on demand, backfilling legacy records, editing GPS metadata, or changing import behavior.
- Showing videos, skipped items, replacement history, unmanaged paths, photos without GPS, malformed GPS, or any original-media path on the map.
- Offline maps, tile prefetch/download, a new map provider, geocoding/search, route planning, map-position persistence, or saved map filters.
- A thumbnail popup, bulk actions from map pins, or changes to the existing preview's media mutation controls.

## Implementation Approach

Extend the existing filtered-library result contract with an optional, validated coordinate field sourced from the encrypted catalogue. This preserves one reactive query and lets the UI derive map points from the already-filtered result set without adding a separate command or request race.

Bundle pinned Leaflet and Leaflet.markercluster JS/CSS assets with the application, load them only while the map view is mounted, and use their standard marker clustering/spiderfy behavior. The application itself remains local-first: only standard OSM tile images requested for the user-visible viewport leave the device. The frontend owns the map/list switch, DOM lifecycle, error reporting, preview handoff, and scroll restoration; native search remains the authority for active imported-media selection and coordinate validation.

## Critical Implementation Details

The map library must be created only after its map container has mounted and must be destroyed when the user returns to the list, locks/closes the library, or the filtered result set is replaced. Recreate or update it only from the latest `search_items` snapshot so stale async search results cannot leave old markers behind.

Do not issue any tile prefetch/cache-control override. Keep OSM attribution visible in the map canvas and report a tile layer error as the chosen recoverable error state; tile requests must use the HTTPS standard endpoint and are limited to user-interactive viewport rendering.

## Phase 1: Add validated GPS data to filtered library results

### Overview

Make every current filtered-library result able to carry an optional safe coordinate, with native validation ensuring only usable imported image coordinates reach the UI. This produces a backend-to-frontend slice that can be manually checked through the existing Library without touching user media.

### Changes Required:

#### 1. Filtered-library DTO and encrypted-catalogue query

**File**: `src-tauri/src/search.rs`

**Intent**: Extend the existing `SearchLibraryItem` response and `query_imported_items` projection to read saved `gps_json` alongside the current active-import data, deserialize it defensively, and expose an optional coordinate only for valid image points.

**Contract**: Preserve all date, media-type, tag, active-import, managed-destination, and replacement predicates used by `search_library`. A coordinate must have finite latitude in `[-90, 90]` and longitude in `[-180, 180]`; null, invalid JSON, wrong-shape, non-finite, or out-of-range values become `None`, never an error or a map point. Videos must return no coordinate even if catalogue data is malformed or unexpectedly present. No schema migration, original-media access, or file read is introduced.

#### 2. Frontend search-result contract

**File**: `src/app.rs`

**Intent**: Mirror the optional coordinate in the deserialized `SearchLibraryItem` shape so the existing reactive search state carries the location data required by a later map view.

**Contract**: The frontend field uses the current latitude/longitude serialization convention and remains optional; existing grid cards, preview selection, date/tag/media filters, and loading/error behavior retain their present behavior in this phase.

#### 3. GPS query and serialization regressions

**Files**: `src-tauri/src/search.rs`, `src/app.rs`

**Intent**: Add fixture-backed native coverage for safe GPS propagation and lightweight frontend contract coverage for optional coordinate deserialization.

**Contract**: Tests demonstrate that only active imported images matching the existing filters expose valid coordinates; null/malformed/out-of-range GPS, videos, skipped entries, and replaced entries do not. Existing EXIF conversion and persistence tests remain intact.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes, including filtered-search coordinate validation and exclusion cases.
- `cargo check --workspace` succeeds with synchronized Rust/Tauri DTOs.

#### Manual Verification:

- Import GPS-tagged and non-GPS photos, then apply date, media-type, and tag filters in the Library; confirm the existing grid and preview behavior is unchanged.
- Reopen the protected library and confirm imported media remains searchable, demonstrating that stored catalogue GPS is read without changing files.

**Implementation Note**: After completing this phase and all automated verification passes, pause for human confirmation of the manual checks before proceeding to the next phase.

---

## Phase 2: Deliver the map/list workspace transition

### Overview

Add the user-visible map mode to the filters workspace, including the Show map entry point, a back-to-list route, empty/error states, and preservation of the current results context. This is a complete vertical slice before marker clustering is added.

### Changes Required:

#### 1. Map-view lifecycle and Library controls

**File**: `src/app.rs`

**Intent**: Add the map/list view state, map loading/error state, and an accessible Show map control in the Library results area. Replace only the grid/empty-result content with the map surface; leave the current filter sidebar and applied-filter controls available.

**Contract**: Opening the map uses the current `search_items` data and filters to image records with valid coordinates. The list is replaced, not overlaid. `Back to list` restores the active filters and captured grid scroll position. A no-coordinate result renders an explanatory empty map state with Back to list. A tile failure renders the selected internet-required error state with the same recovery control. All icon-only controls use the bundled Font Awesome map/back marks plus both `aria-label` and `title`.

#### 2. Preview-context preservation

**File**: `src/app.rs`

**Intent**: Reuse `load_preview` for a selected map item and preserve map mode when the existing preview closes.

**Contract**: A map selection resolves the current result index by candidate ID before invoking the existing preview loader; it never reconstructs media URLs or bypasses `media_details`. Preview close restores the active map instead of resetting the Library. If a reactive search removes the selected item, use the established preview-clear behavior and do not open stale media.

#### 3. Responsive and accessible map workspace presentation

**File**: `assets/styles.css`

**Intent**: Add map toolbar, canvas, empty/error, and compact-screen styles that match the existing Library visual system and leave all controls keyboard reachable.

**Contract**: The map has a stable, usable height at desktop and mobile breakpoints; the visible OSM attribution is not clipped or covered. Focus indicators match the established `.filter-disclosure` and action-button treatment, and empty/error copy remains readable alongside filters.

#### 4. View-state regression coverage

**File**: `src/app.rs`

**Intent**: Extend the existing source-level UI contract tests for the Show map control, back route, valid-GPS empty state, error state, candidate-ID preview handoff, and scroll restoration hooks.

**Contract**: Tests assert durable labels/data hooks and state contracts rather than browser map rendering internals.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with map/list navigation and preview-context source contracts.
- `cargo check --workspace` succeeds without producing a DMG bundle.

#### Manual Verification:

- With active filters, choose Show map and confirm the media grid is replaced while filters remain active; choose Back to list and confirm the same results and grid scroll position return.
- With a filtered set containing no GPS-tagged images, confirm the explanatory empty state and Back to list control work.
- Temporarily disconnect networking or block tile loading, then confirm the map displays a clear internet-required error and returns safely to the list.

**Implementation Note**: After completing this phase and all automated verification passes, pause for human confirmation of the manual checks before proceeding to the next phase.

---

## Phase 3: Render bundled OpenStreetMap tiles and clustered selectable pins

### Overview

Integrate the pinned map renderer and clustering extension as app assets, render the current valid points against interactive OpenStreetMap tiles, and complete the requested cluster/spider-ray selection behavior.

### Changes Required:

#### 1. Pinned local mapping assets and licence notices

**Files**: `assets/vendor/leaflet/*`, `assets/vendor/leaflet.markercluster/*`, `assets/styles.css`

**Intent**: Add reviewed, pinned distribution JS/CSS files and their required licence notices to application assets, with styles imported from local files rather than a CDN.

**Contract**: The renderer and clusterer load from the packaged app. No remote JavaScript, stylesheet, analytics, geocoding, or photo URL is requested. Dependency versions and licensing are recorded with the bundled artifacts; generated build output remains untracked.

#### 2. Map renderer bridge and lifecycle

**File**: `src/app.rs`

**Intent**: Add a small wasm/JavaScript bridge that creates the Leaflet map only after the map canvas exists, installs an HTTPS OpenStreetMap tile layer, fits the current marker bounds, reports tile-layer errors, and tears down listeners/layers cleanly.

**Contract**: The tile URL is `https://tile.openstreetmap.org/{z}/{x}/{y}.png`; visible attribution contains `© OpenStreetMap contributors` with an OSM link. The bridge adds only user-visible viewport tiles and does not prefetch, request offline tiles, alter caching headers, or send coordinates to a service. When there is one point, use a practical single-marker zoom; when there are multiple points, fit bounds with padding. It receives opaque candidate IDs and coordinates, then calls the Rust-provided selection callback without rendering untrusted filenames into HTML.

#### 3. Cluster, spider-ray, and pin selection behavior

**File**: `src/app.rs`

**Intent**: Feed map points into Leaflet.markercluster with nearby clustering and max-zoom spiderfy behavior so photos sharing a location fan out along visible rays and can be chosen independently.

**Contract**: Nearby markers initially show count clusters; zooming/clicking drills into them. At maximum detail, a cluster containing same-coordinate photos spiderfies, each expanded marker selects its own candidate ID, and a single marker opens the existing preview. Configure chunked marker loading to keep a large filtered library responsive; only the current filtered result set is added.

#### 4. Mapping integration contract coverage

**Files**: `src/app.rs`, `assets/styles.css`

**Intent**: Guard the local asset paths, HTTPS tile endpoint, attribution, no-prefetch constraints, fit-bounds/cluster/spiderfy configuration, error callback, and pin-to-preview bridge with focused source contracts.

**Contract**: Automated tests do not require network tiles. The manual pass verifies real interactive rendering, attribution visibility, clustering, same-coordinate ray selection, and the network failure state.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes with native GPS safety plus frontend mapping integration contracts.
- `cargo check --workspace` succeeds and local mapping assets are included by the Dioxus build.

#### Manual Verification:

- In a networked library with geographically distributed GPS-tagged photos, open Show map and confirm it initially fits all matching image markers, visibly credits OpenStreetMap, and does not show non-GPS photos or videos.
- Verify nearby markers cluster, then zoom/click a shared-coordinate cluster until it fans into rays; choose each ray and confirm it opens the corresponding existing media preview.
- Close the preview and confirm the map remains active; Back to list returns the original filtered grid at its former scroll position.

**Implementation Note**: After completing this phase and all automated verification passes, pause for human confirmation of the manual checks before completing the change.

## Testing Strategy

### Unit Tests:

- Validate saved GPS JSON before it crosses the native search boundary, including finite/range checks and image-only behavior.
- Verify active imported-media predicates and every existing date/media/tag filter remain in force when coordinates are selected.
- Verify map view state, candidate-ID-to-preview lookup, list scroll-restoration hooks, tile error handling hooks, local asset references, OSM attribution, cluster/spiderfy options, and teardown hooks.

### Integration Tests:

- Use an encrypted catalogue fixture containing valid GPS images, missing GPS, malformed GPS, videos, skipped decisions, and replacements to exercise the `search_library` response.
- Build/check the Dioxus and Tauri workspace with bundled map assets, without producing a DMG.

### Manual Testing Steps:

1. Import a mixture of GPS-tagged and untagged images, then use dates, tags, and media-type filters before opening the map.
2. Confirm only matching GPS-tagged images are mapped, the initial bounds include them, and nearby and exact-overlap selections work as designed.
3. Open a pin, close the media preview, and return to the grid; confirm map/list mode, filters, and list position are preserved appropriately.
4. Confirm the zero-GPS and tile-error states explain the condition and always offer a safe Back to list path.
5. Verify visible attribution and confirm no map download/prefetch option exists.

## Performance Considerations

The current query returns the filtered list in one request; adding a parsed optional coordinate avoids preview-detail N+1 calls. Marker clustering must use chunked bulk addition so a large filtered set does not freeze the WebView. Map tiles are restricted to the human-visible viewport, and normal WebView HTTP caching is left intact.

## Migration Notes

No catalogue migration or GPS backfill is required. `gps_json` is already nullable and legacy-unlock repair supplies the column; records without valid stored coordinates simply remain absent from the map.

## References

- Current filters plan: `context/changes/filters-ux/plan.md`
- Filtered library UI: `src/app.rs:696-755`, `src/app.rs:1792-1904`
- Native filtered search: `src-tauri/src/search.rs:568-684`
- GPS schema compatibility: `src-tauri/src/library.rs:813`, `src-tauri/src/library.rs:1069-1090`
- [OpenStreetMap Tile Usage Policy](https://operations.osmfoundation.org/policies/tiles/)
- [Leaflet API reference](https://leafletjs.com/reference.html)
- [Leaflet.markercluster documentation](https://github.com/Leaflet/Leaflet.markercluster)

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles.

### Phase 1: Add validated GPS data to filtered library results

#### Automated

- [x] 1.1 `cargo test --workspace` passes, including filtered-search coordinate validation and exclusion cases
- [x] 1.2 `cargo check --workspace` succeeds with synchronized Rust/Tauri DTOs

#### Manual

- [x] 1.3 Existing grid and preview behavior remains unchanged for GPS-tagged and non-GPS imports under active filters
- [x] 1.4 Reopening the protected library preserves searchable imports without changing media files

### Phase 2: Deliver the map/list workspace transition

#### Automated

- [ ] 2.1 `cargo test --workspace` passes with map/list navigation and preview-context source contracts
- [ ] 2.2 `cargo check --workspace` succeeds without producing a DMG bundle

#### Manual

- [ ] 2.3 Map replaces the grid and Back to list restores filters and grid scroll position
- [ ] 2.4 Zero-GPS and tile-loading failure states explain the condition and recover to the list

### Phase 3: Render bundled OpenStreetMap tiles and clustered selectable pins

#### Automated

- [ ] 3.1 `cargo test --workspace` passes with native GPS safety plus frontend mapping integration contracts
- [ ] 3.2 `cargo check --workspace` succeeds and local mapping assets are included by the Dioxus build

#### Manual

- [ ] 3.3 The online map fits matching GPS image points and visibly credits OpenStreetMap
- [ ] 3.4 Nearby markers cluster and same-coordinate photos fan into independently selectable rays
- [ ] 3.5 Preview close preserves the map, and Back to list restores the filtered grid position

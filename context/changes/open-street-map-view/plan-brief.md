# Open street map view — Plan Brief

> Full plan: `context/changes/open-street-map-view/plan.md`

## What & Why

The Library filters workspace will gain a Show map action that replaces the media grid with an interactive OpenStreetMap view. It makes location-aware browsing useful without exposing pictures that lack GPS data or weakening the existing imported-media safety boundary.

## Starting Point

Library filters already reactively query active imported media by dates, type, and tags. GPS is extracted during import and stored locally in the encrypted catalogue, but filtered result cards do not currently receive it, and the application has no map renderer.

## Desired End State

The map shows only matching imported images with valid stored GPS coordinates and fits those points on open. Nearby photos cluster; identical-coordinate photos fan into selectable rays, and each selection opens the established media preview. Returning preserves filters and the grid's scroll position; empty and network-failure states give clear recovery.

## Key Decisions Made

| Decision | Choice | Why |
| --- | --- | --- |
| Tile source | Online standard OpenStreetMap tiles | Delivers a useful basemap without offline-map storage or update complexity. |
| View placement | Replace the Library grid | Keeps the filter workspace present while creating a focused map view. |
| Map scope | All active filters; images with valid GPS only | Makes the map an honest geographic view of current results and excludes videos/no-location media. |
| Initial viewport | Fit all visible pins | Immediately frames the photos relevant to the active filters. |
| Dense locations | Nearby clustering plus spiderfied same-coordinate rays | Maintains readability while allowing the user to choose the exact picture. |
| Pin behavior | Reuse existing media preview | Retains the established safe inspection workflow. |
| Failure behavior | Empty/error map states with Back to list | Keeps navigation predictable and explains missing GPS or internet tiles. |
| Context return | Preserve filters, scroll position, and map after preview close | Supports exploration without losing the user's place. |

## Scope

**In scope:**

- Validated GPS coordinates in the existing filtered-library result contract.
- Bundled Leaflet renderer and clustering assets, interactive OSM tiles, attribution, clusters, spider rays, and pin-to-preview selection.
- Map/list navigation, empty/error states, responsive/accessibility treatment, and regression coverage.

**Out of scope:**

- Offline maps, tile prefetch, GPS editing/backfill, geocoding, route planning, thumbnails on markers, and any access to original media.

## Architecture / Approach

The existing native `search_library` query remains the single source of filtered managed-media results and adds an optional validated coordinate from `item_decisions.gps_json`. Dioxus derives map points from that snapshot, mounts local Leaflet/MarkerCluster assets only in map mode, and sends a selected candidate ID into the existing preview loader. Only user-visible OSM tile images load remotely; no photo or original-media access is added.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Validated GPS search data | Safe optional coordinates on current filtered results | Legacy/malformed catalogue data must never reach the map. |
| 2. Map/list workspace transition | Usable Show map, recovery states, and preserved context | Correct UI/preview/scroll lifecycle handling. |
| 3. OSM renderer and selectable pins | Local map assets, tiles, clusters, and spider rays | Tile-policy compliance and responsive marker performance. |

**Prerequisites:** A GPS-tagged imported-image fixture for manual verification and network access for the online map pass.

**Estimated effort:** ~2–3 implementation sessions across 3 vertical phases.

## Open Risks & Assumptions

- Standard OSM tiles are best-effort and require network access; the application will show an explicit recoverable failure state rather than provide an offline fallback.
- Browser/WebView caching is expected to honor normal tile responses; this feature must not prefetch or download map areas.
- The pinned Leaflet clustering extension's max-zoom spiderfy interaction satisfies the requested sun-ray selection for overlapping photos.

## Success Criteria (Summary)

- Active filters map only matching GPS-tagged imported images; invalid/no-GPS media and videos never appear.
- The map fits points, clusters dense areas, fans overlapping photos into selectable rays, and pins open the existing preview.
- Users can recover from empty/network states and return to the same filtered grid position, with visible OpenStreetMap attribution.

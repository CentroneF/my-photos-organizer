---
project: "Photo Handler"
version: 1
status: draft
created: 2026-08-21
updated: 2026-08-31
prd_version: 1
main_goal: market-feedback
top_blocker: decisions
---

# Roadmap: Photo Handler

> Derived from `context/foundation/prd.md` (v1) and an auto-researched codebase baseline.
> Edit in place; archive when superseded.
> Slices below are listed in dependency order. The “At a glance” table is the index.

## Vision recap

Photo Handler gives a person a local catalogue of their own photos, videos, and prior import decisions, so they can consolidate backups without losing track of duplicates. It connects careful import review with later library search while keeping personal media and its catalogue on the user's computer.

## North star

**S-01: User can set up and reopen a protected managed library.** This is the selected first proof point: it establishes that the app can safely become the trusted home for one person's media catalogue.

> Here, “north star” means the first user-visible milestone chosen to prove that the product is worth continuing to build; it is placed as early as its prerequisites allow.

## At a glance

| ID | Change ID | Outcome (user can …) | Prerequisites | PRD refs | Status |
|---|---|---|---|---|---|
| S-01 | protected-library-setup | set up a fixed, protected managed library and reopen its existing state | — | FR-001 | ready |
| S-02 | choose-import-folder | choose a folder of photos and videos as an import source | S-01 | FR-002 | proposed |
| S-03 | review-and-decide-media | review an import item, tag it, and explicitly import or skip it | S-01, S-02 | US-01, FR-003, FR-004, FR-005 | proposed |
| S-04 | show-duplicate-and-history-context | see possible duplicate or similar media and prior handling context while reviewing | S-03 | US-01, FR-006, FR-007 | complete |
| S-05 | search-managed-library | search managed media by tags and available metadata | S-03 | FR-009 | proposed |

## Streams

Navigation aid — groups items that share a prerequisites chain. Canonical ordering still lives in the dependency graph below; this table is the proposed reading order across parallel tracks.

| Stream | Theme | Chain | Note |
|---|---|---|---|
| A | Trusted import path | `S-01` → `S-02` → `S-03` | Delivers the basic local-media workflow that can gather real user feedback. |
| B | Review context and discovery | `S-04` / `S-05` | Both follow Stream A at `S-03` and can proceed in parallel. |

## Baseline

What is already in place in the codebase as of 2026-08-21 (auto-researched and user-confirmed). The slices below extend this scaffold rather than re-scaffolding it.

- **Frontend:** partial — application shell and starter screen exist in `src/main.rs:7` and `src/app.rs:22`.
- **Backend / API:** partial — native command registration exists, with only a starter command in `src-tauri/src/lib.rs:2` and `src-tauri/src/lib.rs:11`.
- **Data:** absent — no persistence dependency, schema, or migration evidence; see `src-tauri/Cargo.toml:20`.
- **Auth:** absent — no credential, session, recovery, or guard flow is present.
- **Deploy / infra:** partial — desktop bundle configuration exists in `src-tauri/tauri.conf.json:6` and `src-tauri/tauri.conf.json:25`; release automation is not wired.
- **Observability:** partial — INFO-level application logging is initialized in `src/main.rs:5`; no broader error tracking or metrics are present.

## Foundations

No standalone foundation is proposed. The absent data and access-control layers are introduced inside S-01, the first user-visible flow that needs them; this keeps the work end-to-end and manually verifiable.

## Slices

### S-01: Set up a protected managed library

- **Outcome:** User can choose the fixed folder for their managed library, protect access with a password, and reopen existing application state in that folder.
- **Change ID:** protected-library-setup
- **PRD refs:** FR-001
- **Prerequisites:** —
- **Parallel with:** —
- **Blockers:** —
- **Unknowns:**
  - How should password recovery work while preserving the local-only requirement? — Owner: user. Block: no.
- **Risk:** The first trust boundary must preserve existing library data and never turn a setup action into media deletion.
- **Status:** ready

### S-02: Choose an import folder

- **Outcome:** User can choose a folder of photos and videos as the source for an import session.
- **Change ID:** choose-import-folder
- **PRD refs:** FR-002
- **Prerequisites:** S-01
- **Parallel with:** —
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Source selection must remain distinct from the fixed managed-library location so original media stays untouched.
- **Status:** proposed

### S-03: Review, tag, and decide on media

- **Outcome:** User can review each item from an import folder, add tags, and explicitly import or skip it.
- **Change ID:** review-and-decide-media
- **PRD refs:** US-01, FR-003, FR-004, FR-005
- **Prerequisites:** S-01, S-02
- **Parallel with:** —
- **Blockers:** —
- **Unknowns:**
  - What acceptance criteria define a complete import-folder review? — Owner: user. Block: no.
- **Risk:** This is the first complete review loop, so its explicit user decision must be reliable before richer suggestions are added.
- **Status:** proposed

### S-04: Show duplicate, similarity, and prior-decision context

- **Outcome:** User can see already handled items and possible similar media while reviewing, with prior import and skip decisions remembered.
- **Change ID:** show-duplicate-and-history-context
- **PRD refs:** US-01, FR-006, FR-007
- **Prerequisites:** S-03
- **Parallel with:** S-05
- **Blockers:** —
- **Unknowns:** —
- **Risk:** Suggestions must inform the user without replacing their final import, skip, or deletion decision.
- **Status:** complete
- **Note:** Visual similarity is intentionally limited to supported still images. Visual video similarity remains a separately framed future decoder/distribution decision; videos retain exact-history context only.

### S-05: Search the managed library

- **Outcome:** User can find managed media by tags and available metadata.
- **Change ID:** search-managed-library
- **PRD refs:** FR-009
- **Prerequisites:** S-03
- **Parallel with:** S-04
- **Blockers:** —
- **Unknowns:**
  - What measurable target defines successful completion of review, import, and library search? — Owner: user. Block: no.
- **Risk:** Search needs imported records from the review loop, but should not depend on similarity suggestions to deliver discovery value.
- **Status:** proposed

## Backlog Handoff

| Roadmap ID | Change ID | Suggested issue title | Ready for `/10x-plan` | Notes |
|---|---|---|---|---|
| S-01 | protected-library-setup | Set up and reopen a protected managed library | yes | Selected first proof point. |
| S-02 | choose-import-folder | Let users choose an import folder | no | Requires S-01. |
| S-03 | review-and-decide-media | Let users review, tag, import, or skip media | no | Requires S-01 and S-02. |
| S-04 | show-duplicate-and-history-context | Show duplicate, similarity, and decision context | no | Requires S-03. |
| S-05 | search-managed-library | Search managed media by tags and metadata | no | Requires S-03; can run alongside S-04. |

## Open Roadmap Questions

1. **What are the acceptance criteria for reviewing an import folder?** — TBD by user. Block: no.
2. **What measurable target defines successful completion of the review, import, and library-search flow?** — TBD by user. Block: no.

## Parked

- **Deleting original source files after import** — Why parked: PRD Non-Goals keeps originals untouched in the MVP.
- **Location-map view** — Why parked: FR-010 is a post-MVP enhancement.
- **Multi-user and shared-library capabilities** — Why parked: PRD Non-Goals limits the MVP to one personal library.

## Done

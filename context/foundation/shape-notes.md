---
project: "Photo Handler"
context_type: greenfield
product_type: desktop
target_scale:
  users: small
timeline_budget:
  mvp_weeks: 10
  hard_deadline: null
  after_hours_only: true
created: 2026-08-21
updated: 2026-08-21
checkpoint:
  current_phase: 7
  phases_completed: [1, 2, 3, 4, 5, 6]
  gray_areas_resolved: []
  frs_drafted: 10
  quality_check_status: pending
---

## Vision & Problem Statement

People who keep personal photos and videos across multiple sources and backups cannot easily tell which files are duplicates or find media they have already saved. The result is wasted disk space and a personal library that is difficult to search.

The product keeps a local catalogue of a person's own media and their import decisions, making duplicate handling and search part of the same personal-library workflow.

## User & Persona

### Primary persona

An individual managing their own personal photo and video collection. They reach for the product when consolidating backups or trying to find a specific item in a large collection gathered from multiple sources.

## Access Control

One personal library with no roles or sharing. A password protects access; the user's email is used only for password recovery.

## Success Criteria

### Primary

- A person can complete the full photo and video review and import flow, then find managed media through library search.

### Secondary

- A person can view imported photos on a map using their location information.

### Guardrails

- Original files are never deleted unless the user explicitly asks.

## Timeline acknowledgment

Acknowledged on 2026-08-21: 10-week MVP requires sustained dedication; user accepted.

## Functional Requirements

### Import and review

- FR-001: User can choose the fixed folder for their managed media library on first launch; if it already contains application data, the app can read its existing state. Priority: must-have
  > Socrates: Counter-argument considered: a user could choose a library folder repeatedly. Resolution: the folder is fixed during first launch; existing application data in it is loaded.
- FR-002: User can choose a folder of photos and videos to import. Priority: must-have
  > Socrates: Counter-argument considered: selecting an import folder may be unnecessary in the MVP. Resolution: kept; it stands as written.
- FR-003: User can review each photo or video from an import folder. Priority: must-have
  > Socrates: Counter-argument considered: a batch-review flow could be enough. Resolution: kept; individual review stands as written.
- FR-004: User can add tags to a reviewed item. Priority: must-have
  > Socrates: Counter-argument considered: tagging could happen only after import. Resolution: kept in the review flow.
- FR-005: User can import or skip a reviewed item. Priority: must-have
  > Socrates: Counter-argument considered: the app could decide automatically in some cases. Resolution: the user always makes the import-or-skip choice.
- FR-006: User can be shown items already handled and possible similar media while reviewing an item. Priority: must-have
  > Socrates: Counter-argument considered: visual-similarity detection could ship later, retaining only exact duplicates. Resolution: both exact-duplicate and visual-similarity handling ship in this MVP.
- FR-007: User can have prior import and skip decisions remembered, including metadata and hashing for skipped files. Priority: must-have
  > Socrates: Counter-argument considered: only imported files could be remembered. Resolution: skipped files are also remembered with metadata and hashing.

### Library discovery

- FR-009: User can search their managed library by tags and available metadata. Priority: must-have
  > Socrates: Counter-argument considered: tag-only search could be sufficient. Resolution: both tag and metadata search are required.
- FR-010: User can view imported photos on a map using location information. Priority: nice-to-have
  > Socrates: Counter-argument considered: map view may not be needed for the first release. Resolution: post-MVP enhancement.

## User Stories

### US-01: Review an import folder

- **Given** a folder containing media files
- **When** the user starts an import
- **Then** they can tag items and see duplicate and similar media while reviewing them

## Business Logic

During media review, the app suggests actions using exact matches, visual similarity, and records of previously handled media; the user makes the final decision.

The rule uses the media being reviewed and the user's prior handling records. Its output is a suggestion shown as part of the review flow, not an automatic import, skip, or deletion decision.

## Non-Functional Requirements

- Personal media and its catalogue never leave the user's computer.
- The MVP supports macOS and Windows.

## Non-Goals

- Deleting original source files after import is out of scope for the MVP; originals remain untouched.
- A location-map view is post-MVP.
- Multi-user and shared-library capabilities are out of scope; the MVP serves one person's personal library.

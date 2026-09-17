---
date: 2026-09-14T14:35:03+02:00
researcher: Codex
git_commit: 2e02026680c0837c56fbc98205114f65391aa2bc
branch: google-photos-import
repository: CentroneF/my-photos-organizer
topic: "Connect to Google Photos to import pictures with metadata"
tags: [research, google-photos, google-photos-picker, oauth, import, metadata]
status: complete
last_updated: 2026-09-14
last_updated_by: Codex
last_updated_note: "Added repeated-Takeout duplicate behavior and Google cleanup boundary"
---

# Research: Connect to Google Photos to import pictures with metadata

**Date**: 2026-09-14T14:35:03+02:00  
**Researcher**: Codex  
**Git Commit**: `2e02026680c0837c56fbc98205114f65391aa2bc`  
**Branch**: `google-photos-import`  
**Repository**: `CentroneF/my-photos-organizer`

## Research Question

Connect the desktop app to Google Photos so the user can import photos with metadata.

## Summary

The requested full-account import is not technically available to a new third-party Google Photos integration. Since March 2025, Google has removed third-party access to a user's non-app-created library through the former Library API scopes. The supported path is the **Google Photos Picker API**: the user explicitly selects up to 2,000 items in Google Photos, and the app downloads those selections for review and local import. [Google's authorization overview](https://developers.google.com/photos/overview/authorization) and [Picker guide](https://developers.google.com/photos/picker/guides/get-started-picker) document this change.

The product scope should therefore be renamed and planned as **“Import user-selected Google Photos items”**, not a Google Photos sync or whole-library import. It must be transparent that images downloaded through Picker retain EXIF **except location metadata**, while videos are a high-quality transcode rather than guaranteed original bytes. Album membership, descriptions, GPS, and full Google Photos organization are unavailable. [Picker media retrieval](https://developers.google.com/photos/picker/guides/media-items) and the [PickedMediaItem schema](https://developers.google.com/photos/picker/reference/rest/v1/mediaItems) define these limits.

The least risky local architecture is: browser OAuth with PKCE → Picker session → user selection → native download to a private staging area → the existing explicit review/import flow → managed local copy. The app must never request write/delete permissions or mutate Google Photos.

## Detailed Findings

### Google API feasibility and product boundary

- The old `photoslibrary.readonly`, `photoslibrary`, and sharing scopes no longer provide third-party access to a user's existing Google Photos library. Current Library API read/list/search access is limited to media or albums created by the app. Google directs user-library selection use cases to Picker. [Authorization changes](https://developers.google.com/photos/overview/authorization).
- Picker requires only `https://www.googleapis.com/auth/photospicker.mediaitems.readonly`. It creates a selection session, opens the returned `pickerUri` in the system browser, polls according to Google's response, pages selected items (at most 100 per request), downloads while URLs are valid, then deletes the session. [Session lifecycle](https://developers.google.com/photos/picker/guides/sessions), [selected-media listing](https://developers.google.com/photos/picker/reference/rest/v1/mediaItems/list).
- One Picker session supports at most 2,000 selected items; the Picker page cannot be embedded in a WebView and is single-use after selection. [Session resource](https://developers.google.com/photos/picker/reference/rest/v1/sessions).
- Picker returns a stable item ID, create time, filename, MIME type, dimensions, plus some camera or video fields. It does not return albums, descriptions, location, or Google Photos organization. [Picked media schema](https://developers.google.com/photos/picker/reference/rest/v1/mediaItems).
- Image downloads using `=d` retain EXIF except location metadata. Video downloads using `=dv` are high-quality transcoded bytes rather than originals. Base URLs expire after about 60 minutes and require the OAuth bearer token, so they must not be persisted. [Picker media downloads](https://developers.google.com/photos/picker/guides/media-items).

### Existing import and review architecture

- The native command surface begins with the path-only `start_review` request at [`src-tauri/src/lib.rs:8`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/src/lib.rs#L8) and command registration at [`src-tauri/src/lib.rs:324`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/src/lib.rs#L324). A remote source cannot be represented by the current `folder_path` DTO without an explicit source/provider model.
- The selected source is a locally persisted folder pointer; it validates and canonicalizes a filesystem directory, rejects library overlap, then writes `selected-import-source.json`. See [`src-tauri/src/import_source.rs:47`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/src/import_source.rs#L47) and [`src-tauri/src/import_source.rs:156`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/src/import_source.rs#L156).
- Review recursively enumerates a local directory and records a source path plus relative file path; import reconstructs the source file and copies it into the managed library. See [`src-tauri/src/review.rs:447`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/src/review.rs#L447) and [`src-tauri/src/review.rs:1157`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/src/review.rs#L1157). This makes a private downloaded staging folder the safest compatibility bridge, but that staging path must never be presented as the user’s original source.
- The current review metadata model includes filesystem size/timestamps, dimensions, EXIF captured date/camera/orientation, and GPS. It parses local file EXIF directly. See [`src-tauri/src/review.rs:53`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/src/review.rs#L53) and [`src-tauri/src/review.rs:778`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/src/review.rs#L778). The catalogue does not yet persist complete source metadata or remote provenance; a migration is needed.
- Preview asset URLs are granted only for a local source path, so expiring Google URLs cannot be directly substituted into the existing preview mechanism. See [`src-tauri/src/review.rs:369`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/src/review.rs#L369).
- The frontend already separates import-source, review-state, and review-item signals at [`src/app.rs:550`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src/app.rs#L550), which is a reasonable seam for source-kind and provenance DTO additions.

### Security, privacy, and metadata design

- Keep OAuth, HTTP requests, token refresh, and downloads entirely in native Rust. The Dioxus frontend should receive only non-secret status/result DTOs through its existing `invoke` boundary at [`src/app.rs:10`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src/app.rs#L10).
- Use installed-app OAuth in the system browser, with PKCE S256, high-entropy `state`, a random-port loopback callback (or exact registered custom scheme), exact redirect validation, cancellation/expiry cleanup, and no embedded user agent. [Google OAuth for desktop apps](https://developers.google.com/identity/protocols/oauth2/native-app).
- Add native HTTP, OAuth, and OS credential-store dependencies. Existing dependencies contain local encryption/media tooling but no HTTP/OAuth/keychain client ([`src-tauri/Cargo.toml:20`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/src-tauri/Cargo.toml#L20)). Store refresh tokens only in Keychain/Credential Manager; the encrypted catalogue can store non-secret account and media-provenance identifiers.
- Retain per-item Google provenance: provider, stable Picker ID, Picker create time, original filename/MIME, reported dimensions/camera/video fields, local downloaded-content hash, and availability/origin of each metadata field. Do not retain tokens, `pickerUri`, or expiring base URLs.
- The PRD is local-first and forbids removal/movement of originals without explicit authorization. A cloud import is a deliberate retrieval exception and needs a clear privacy disclosure, read-only scope, and Disconnect action that only forgets local credentials/session state. [`context/foundation/prd.md:84`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/context/foundation/prd.md#L84), [`context/foundation/prd.md:97`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/context/foundation/prd.md#L97).

## Architecture Insights

1. Model Google Photos as a distinct remote provider, not a fake filesystem folder.
2. Materialize selected bytes into an app-private, cleaned-up staging directory before passing them to review. This reuses the tested local review and safe copy behavior.
3. Extend review/catalogue models for provenance so the app can show Google-reported metadata honestly without claiming that unavailable GPS or original video bytes were preserved.
4. Treat selection, download, and later review decisions as separate resumable states. Picker sessions and base URLs expire; review/import must remain safe if an OAuth session is cancelled or expires.
5. Keep the initial delivery a manually verifiable vertical slice: connect, select a small batch, download one item, review it, import it, confirm Google remains unchanged, and surface limitations in the UI.

## Historical Context

- [`context/changes/choose-import-folder/plan.md`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/context/changes/choose-import-folder/plan.md) established a non-mutating local source-selection contract. Google must be a new source type, not a hidden expansion of that contract.
- [`context/changes/review-and-decide-media/plan.md`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/context/changes/review-and-decide-media/plan.md) established explicit import/skip decisions and safe managed-copy behavior; retain this gate for downloaded Google items.
- [`context/foundation/lessons.md`](https://github.com/CentroneF/my-photos-organizer/blob/2e02026680c0837c56fbc98205114f65391aa2bc/context/foundation/lessons.md) requires plans to be vertical and manually verifiable from the frontend.

## Related Research

No prior Google Photos integration research exists in this repository.

## Open Questions

1. Is user-selected Picker import acceptable, given that full-library sync and albums are unavailable through Google’s current public API?
2. If archive-grade originals, GPS, album membership, and sidecar metadata are mandatory, should the product instead guide users to Google Takeout and reuse the existing local-folder import?
3. Which account identity should be displayed locally, and should the first release support one connected Google account only?
4. What staging-size limits, retention period, and retry UX are acceptable for large selections and expiring Picker URLs?

## Follow-up Research 2026-09-14T14:35:03+02:00

Two separately downloaded and extracted Takeout folders are recognized as an
exact duplicate when the media-file bytes are identical: the existing review
flow persists a BLAKE3 digest and queries it across every prior review session,
regardless of source folder, filename, or filesystem timestamp. A changed EXIF
block, re-encode, edit, or video representation changes that digest. For
supported still images, the existing perceptual comparison can then present an
advisory possible-similar result; it does not cover HEIC or video.

Takeout JSON sidecars are currently ignored, so their GPS/provenance data is
not yet associated with a media candidate. Google Photos APIs cannot delete or
modify ordinary existing user library media, so the product can only generate a
manual cleanup queue rather than promise remote deletion.

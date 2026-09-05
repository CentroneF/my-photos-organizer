# Photo Organizer identity implementation plan

## Overview

Rename the product-facing desktop application from its starter and "Photo Handler" names to **Photo Organizer**, replace the stock Tauri artwork with the selected colorful photo-landscape-and-magnifier icon, and give the installation a new Tauri identifier. The identifier change intentionally starts with empty app settings; it does not migrate remembered-library or import-source state.

## Current State Analysis

The bundle and main window still expose `bootstrap-scaffold`, while the UI and native error messages use `Photo Handler`. Tauri's explicit icon list is present but every referenced asset is stock scaffold artwork.

## Desired End State

On macOS and Windows, the installed app, native window, and web document title display **Photo Organizer**. A colorful but small-size-legible photo-landscape-and-magnifier mark is used consistently for all bundle icon formats. The new identifier creates a fresh app-data namespace: the first launch has no remembered library or import source, while existing user media and library folders are never changed.

### Key Discoveries:

- `src-tauri/tauri.conf.json:3`, `:5`, and `:16` separately control product name, application identifier, and native window title.
- `src-tauri/tauri.conf.json:29`–`:38` references the core bundle icons; `src-tauri/icons/` additionally contains Windows tile and store assets that should be regenerated with the same master artwork.
- `src-tauri/src/lib.rs:100`–`:239` obtains settings through `app.path().app_data_dir()`, so renaming the identifier deliberately changes where remembered library and import-source state are found.
- User-visible branding is concentrated in `src/app.rs:676`, `:1263`–`:1265`, `:1283`, `:1292`, `:1340`, and `:1458`, with corresponding native copy in `src-tauri/src/library.rs:422`, `:462`, `:503`, `:841`, `:847`, `:853`, and `:981`.

## What We're NOT Doing

- Migrating, copying, deleting, or otherwise touching data in the old application-data location.
- Changing a protected library's on-disk marker, catalogue schema, password, or media files.
- Renaming repository documentation or foundation history.
- Adding an auto-import, search, or duplicate-detection capability beyond the visual icon metaphor.

## Implementation Approach

Treat the rename as a complete runtime identity change: rename the native package/executable and library crate because macOS exposes the executable name during `cargo tauri dev`; synchronize the root Cargo and Dioxus application names to avoid leaving starter identity in build output. Use one approved, source-controlled square master image to generate every Tauri icon variant; preserve the selected concept's colored landscape and magnifier while simplifying shapes for 16–32 px recognition. Set `com.fcentron.photo-organizer` as the identifier and make the first-run result explicit through manual verification, without a migration path.

## Critical Implementation Details

The changed identifier must be tested as a clean settings namespace, not as evidence that an existing protected library was removed. The library itself is stored in a user-selected folder; only the remembered pointer and import-source preference live under Tauri's identifier-derived application-data directory.

## Phase 1: Deliver the Photo Organizer product identity

### Overview

Make the desktop product consistently say Photo Organizer and establish the intentional fresh-start boundary through the identifier change.

### Changes Required:

#### 1. Native bundle and web identity

**Files**: `Cargo.toml`, `Cargo.lock`, `Dioxus.toml`, `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json`

**Intent**: Give the installed app, native executable, window, web document, and build metadata the final Photo Organizer identity, and place settings under the new application identifier.

**Contract**: `productName`, the primary window `title`, and `[web.app] title` equal `Photo Organizer`; bundle `identifier` changes from the bootstrap value to `com.fcentron.photo-organizer`. The native package/executable becomes `photo-organizer`, its library crate becomes `photo_organizer_lib`, and `src-tauri/src/main.rs` imports that synchronized library name. The root Cargo package and Dioxus application names become `photo-organizer-ui`; `Cargo.lock` records the renamed packages. No code reads or migrates the old identifier's app-data directory.

#### 2. Frontend and native user-facing wording

**Files**: `src/app.rs`, `src-tauri/src/library.rs`

**Intent**: Replace visible Photo Handler references with Photo Organizer, including picker titles, onboarding branding, setup guidance, similarity settings, validation, and success/error messages.

**Contract**: Every product name displayed by the running UI or returned to it from native library operations is `Photo Organizer`; preserve existing message meaning and the safety promise that user media is never modified. Do not rename hidden on-disk state names merely because their messages change.

### Success Criteria:

#### Automated Verification:

- `rg -n 'bootstrap-scaffold|Photo Handler' Cargo.toml Cargo.lock Dioxus.toml src src-tauri` finds no remaining starter or prior product identity.
- `cargo check --workspace` completes successfully without generating a DMG.
- `cargo test --workspace` completes successfully.

#### Manual Verification:

- Launching `cargo tauri dev` shows Photo Organizer in the native title bar and onboarding screen.
- A newly created or reopened protected library still shows the updated success, validation, and settings copy without implying media was changed.
- With the new identifier's app-data location, first launch offers setup rather than silently restoring the prior remembered-library or import-source selection; an existing library can still be selected and opened manually.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Package the selected Photo Organizer icon

### Overview

Create a polished production master from the selected colorful photo-landscape-and-magnifier direction and regenerate all desktop icon formats from it.

### Changes Required:

#### 1. Source artwork and generated Tauri icon family

**Files**: `src-tauri/icons/` (new source master and regenerated platform assets), `src-tauri/tauri.conf.json` (only if icon paths must change)

**Intent**: Replace the generic Tauri loop icon with one cohesive Photo Organizer mark across the formats used by macOS and Windows packaging.

**Contract**: The source master is a square, transparent-background colorful landscape/photo mark with an overlaid magnifier; it contains no text, letters, watermark, or tiny details that vanish at 16–32 px. Generate and commit `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`, and the Windows tile/store variants from that same master. Keep the bundle icon list pointing at valid generated files.

### Success Criteria:

#### Automated Verification:

- The Tauri icon generator completes successfully from the committed master source.
- `file src-tauri/icons/icon.icns src-tauri/icons/icon.ico src-tauri/icons/32x32.png src-tauri/icons/128x128.png` reports valid platform/icon image formats.
- `cargo tauri build --bundles app` completes successfully without creating a DMG.

#### Manual Verification:

- The application uses the new colorful icon in the running desktop window/application switcher where the platform exposes it.
- The icon remains recognizable as a photo with a magnifier at small file-list or shortcut sizes and does not show a non-transparent rectangular background.
- macOS and Windows target asset sets contain the same visual identity rather than stock Tauri artwork.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Verify the renamed first-run desktop experience

### Overview

Perform a user-visible release-readiness pass that proves the renamed app behaves as a fresh installation without endangering existing libraries.

### Changes Required:

#### 1. First-run and existing-library verification record

**Files**: `context/changes/app-icon-and-name/plan.md` (Progress only, after verification)

**Intent**: Record that the intentional no-migration outcome and the product identity were checked through the actual desktop frontend.

**Contract**: No implementation files are added for migration. Verification demonstrates that the app can begin fresh under the new identifier and that selecting an existing protected library remains an explicit, safe operation.

### Success Criteria:

#### Automated Verification:

- `cargo test --workspace` passes after the completed branding and asset changes.
- `cargo check --workspace` passes after the completed branding and asset changes.

#### Manual Verification:

- Start from the new identifier's empty app-data state and confirm the setup flow has no remembered-library or import-source state.
- Use the frontend to choose and unlock an existing protected library; confirm its original media and catalogue remain intact.
- Confirm the product name and selected icon remain consistent through the title bar, onboarding, and packaged application artifact.

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before concluding the change.

---

## Testing Strategy

### Unit Tests:

- Retain and run the existing library tests to prove branding/config edits do not alter protected-library behavior.
- Add tests only if an identity-related code path gains new behavior; no settings migration is intentionally introduced.

### Integration Tests:

- Build the native app bundle for the app target only and inspect the generated icon formats.

### Manual Testing Steps:

1. Start the renamed app with an empty new identifier-derived app-data directory and verify the setup screen and title show Photo Organizer.
2. Create or open a library through the frontend and confirm product copy is updated while source media remains untouched.
3. Restart the app to confirm its fresh-start boundary; then explicitly choose an existing library and unlock it safely.
4. Inspect the application/shortcut icon at small and normal sizes on the available platform.

## Performance Considerations

No runtime performance change is expected. Icon conversion happens at build time; the bundled assets are loaded by the operating system.

## Migration Notes

This change intentionally has no migration. The new identifier creates a separate Tauri application-data directory, so remembered library and import-source preferences are not carried forward. Existing user-selected library folders and their protected catalogue files remain untouched and may be re-opened manually.

## References

- Bundle identity and icon configuration: `src-tauri/tauri.conf.json:3`
- Web title: `Dioxus.toml:8`
- Application-data access: `src-tauri/src/lib.rs:100`
- Frontend product copy: `src/app.rs:1263`
- Native library product copy: `src-tauri/src/library.rs:422`
- Product guardrail: `context/foundation/prd.md:31`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles.

### Phase 1: Deliver the Photo Organizer product identity

#### Automated

- [x] 1.1 Verify no `bootstrap-scaffold` or `Photo Handler` identity remains in the runtime project metadata or source. — bee491a
- [x] 1.2 Run `cargo check --workspace` without generating a DMG. — bee491a
- [x] 1.3 Run `cargo test --workspace`. — bee491a

#### Manual

- [x] 1.4 Verify the running UI and native title show Photo Organizer. — bee491a
- [x] 1.5 Verify first-run fresh settings and safe manual reopening of an existing library. — bee491a

### Phase 2: Package the selected Photo Organizer icon

#### Automated

- [x] 2.1 Regenerate the complete Tauri icon family from the committed master artwork.
- [x] 2.2 Validate core generated icon file formats.
- [x] 2.3 Run `cargo tauri build --bundles app` without creating a DMG.

#### Manual

- [x] 2.4 Verify the new icon is recognizable and transparent at desktop sizes.
- [x] 2.5 Verify macOS and Windows asset sets use the same custom identity.

### Phase 3: Verify the renamed first-run desktop experience

#### Automated

- [ ] 3.1 Run `cargo test --workspace` after all changes.
- [ ] 3.2 Run `cargo check --workspace` after all changes.

#### Manual

- [ ] 3.3 Verify a clean first run under the new identifier.
- [ ] 3.4 Verify an existing protected library can be explicitly selected and unlocked without media changes.
- [ ] 3.5 Verify consistent Photo Organizer branding and icon in the desktop experience.

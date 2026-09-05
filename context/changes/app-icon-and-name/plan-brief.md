# Photo Organizer identity — Plan Brief

> Full plan: `context/changes/app-icon-and-name/plan.md`

## What & Why

The desktop app will become **Photo Organizer**, replacing the remaining starter identity and the current Photo Handler wording users see. It will also receive a colorful photo-landscape-and-magnifier app icon that communicates photo discovery and organization.

## Starting Point

The Tauri product name and native window still say `bootstrap-scaffold`; the frontend and native messages say Photo Handler. The repository currently packages stock Tauri icon assets, and settings are resolved from a Tauri identifier-derived application-data directory.

## Desired End State

The installed app, native title bar, web title, interface, and native messages consistently say Photo Organizer. macOS and Windows use a complete custom icon family derived from the selected colorful concept. The new bundle identifier intentionally starts with clean remembered settings while leaving existing protected libraries and all media untouched.

## Key Decisions Made

| Decision | Choice | Why |
| --- | --- | --- |
| Product name | Photo Organizer | It describes the app's central purpose clearly. |
| Rename scope | Product-facing surfaces | Delivers a coherent user experience without unrelated repository-history cleanup. |
| Bundle identifier | `com.fcentron.photo-organizer` | Removes scaffold identity from the installed application. |
| Existing settings | Fresh start, no migration | Chosen to avoid data-copy complexity; existing libraries remain manually reopenable. |
| Icon direction | Colorful photo landscape + magnifier | It makes photo discovery recognizable and gives the product a distinctive visual identity. |
| Icon delivery | Complete Tauri icon family | Keeps macOS and Windows bundle assets consistent. |

## Scope

**In scope:**

- Native, web, frontend, and native-message naming visible to users.
- New bundle identifier with explicitly fresh settings.
- Production master artwork plus regenerated macOS and Windows icon assets.
- Desktop build and first-run verification without a DMG.

**Out of scope:**

- Migration of remembered settings from the old identifier.
- Any change to library catalogue data, protected-library markers, or user media.
- Renaming internal crates, repository documentation, or historical planning documents.

## Architecture / Approach

Configuration provides the installed name, window title, identifier, and bundle icon references; Dioxus provides the document title and visible branding; native library operations return several user-facing messages. The identifier change creates a new app-data namespace, while the library itself remains in the folder the user explicitly selects. One transparent square source image produces every platform-specific icon output.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Product identity | Consistent Photo Organizer naming and the new fresh-start identifier | Confusing a settings reset with data loss |
| 2. Icon packaging | Custom master artwork and all generated platform icons | Poor legibility at small sizes |
| 3. Desktop verification | Confirmed fresh first run and safe explicit library reopening | Platform-specific icon display differences |

**Prerequisites:** Tauri and Dioxus CLIs available; a macOS or Windows environment to inspect the installed/running icon.
**Estimated effort:** ~2–3 focused sessions across 3 phases.

## Open Risks & Assumptions

- Old remembered-library and import-source preferences will not appear after the identifier rename by design.
- The selected rich icon must be simplified during production work if its smallest generated size loses recognition.
- A platform may cache an old icon temporarily; verification should distinguish cache refresh from a packaging mistake.

## Success Criteria (Summary)

- All user-visible naming says Photo Organizer in the running desktop app.
- The full icon family is valid, custom, transparent, and recognizable at small sizes.
- Fresh start and manual opening of an existing protected library work without modifying user media.

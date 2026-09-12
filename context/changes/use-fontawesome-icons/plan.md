# Implementation Plan: Use Font Awesome Icons

## Current State Analysis

Font Awesome Free is already bundled under `assets/fontawesome/` and loaded by
`src/app.rs`. The single-media preview toolbar already uses `fa-solid` icons,
but several icon-only controls and decorative marks elsewhere still render
Unicode glyphs. These include folder navigation decoration, the library
settings trigger, filter disclosures, and removal marks in filter and tag
chips.

`AGENTS.md` currently defines repository and implementation conventions but
does not prescribe an icon source. There is also no regression coverage that
would alert a future implementation when it introduces one of the replaced
ad-hoc glyphs.

## Desired End State

- Every icon-only control and decorative mark in scope uses the locally bundled
  Font Awesome Free icons.
- Icon-only interactive controls expose an `aria-label` and a `title` tooltip.
- Text-labelled controls remain text-labelled; only their decorative/removal
  marks are replaced where applicable.
- `AGENTS.md` directs future agents to the local Font Awesome convention.
- A focused source-level test prevents the known replaced glyph patterns from
  returning and verifies the convention's essentials.

## What We Are Not Doing

- Replacing icons embedded in text-labelled workflow buttons.
- Introducing a CDN, additional icon dependency, or a custom icon system.
- Altering layout, navigation behavior, filtering behavior, or tag-removal
  behavior beyond the visual icon markup and its accessibility metadata.
- Treating ordinary content and media-metadata characters as UI icons.

## Implementation Approach

Use the existing local Font Awesome stylesheet and the same `fa-solid` markup
pattern that is already proven in the preview toolbar. Replace only the
identified icon-only/decorative glyphs, preserve labels and event handlers,
and give icon-only buttons an accessible name plus tooltip. Record the rule in
the repository guidance and add small source-level assertions that are narrow
enough to avoid policing unrelated user-facing text.

## Phase 1: Standardize Scoped Icons and Enforce the Convention

### Overview

Deliver the complete user-visible icon refresh together with accessibility,
future-agent guidance, and regression coverage in one verifiable slice.

### Changes Required

1. **Replace scoped UI glyphs in `src/app.rs`**
   - Replace the folder-picker decorative mark with `fa-folder` and its
     navigation arrow with `fa-arrow-right`.
   - Replace the library settings trigger glyph with `fa-gear` while retaining
     its current click behavior.
   - Replace expanded/collapsed filter disclosure `+`/`−` glyphs with matching
     `fa-chevron-down`/`fa-chevron-up` icons.
   - Preserve filter-chip text, replacing only their trailing removal mark with
     `fa-xmark`.
   - Replace review-tag and preview-tag icon-only removal marks with
     `fa-xmark`.
   - Where an icon-only button does not already provide both forms of context,
     add an `aria-label` and `title` that describe its action. Mark purely
     decorative nested icon elements as hidden from assistive technology.
   - Preserve all event handlers, keyboard behavior, titles that already
     describe an action, and text-labelled controls.

2. **Adjust focused styles in `assets/styles.css`**
   - Make the new icon elements align and size consistently with the existing
     control geometry rather than relying on text-glyph line height.
   - Reuse existing preview-toolbar Font Awesome conventions where possible.
   - Preserve hover, focus-visible, danger, disabled, and compact-layout
     styling of the surrounding buttons and chips.

3. **Document the durable convention in `AGENTS.md`**
   - State that icon-only controls and decorative marks must use the bundled
     local Font Awesome Free assets and `fa-solid` classes.
   - Prohibit new CDN or icon-package additions for this convention unless the
     user explicitly requests a change.
   - Require `aria-label` and `title` on interactive icon-only controls, while
     retaining text labels for text-labelled actions.

4. **Add lightweight regression coverage in `src/app.rs` tests**
   - Extend the existing source-level UI test module with an assertion that
     checks the local Font Awesome stylesheet and expected scoped icon classes.
   - Assert that the specifically replaced Unicode patterns are absent, without
     asserting against unrelated ordinary text or media metadata.
   - Assert that the repository guidance contains the Font Awesome and
     icon-only accessibility rule so the contract is visible to future agents.

### Acceptance Criteria

- Folder navigation, library settings, filter disclosure, filter removal,
  review-tag removal, and preview-tag removal render Font Awesome Free icons;
  no square/missing-glyph fallback appears.
- Filter chips keep their readable text labels.
- Every interactive icon-only control in scope has a meaningful tooltip and
  accessible name.
- Existing preview toolbar icons continue to render and behave unchanged.
- The repository instructions clearly require the local Font Awesome approach.
- Regression tests fail if the known replaced glyph patterns or required
  convention drift reappear.

### Verification

Automated:

- `cargo fmt --check`
- `cargo test --workspace`

Manual:

1. Run `cargo tauri dev` from the repository root.
2. On the folder picker, verify the folder and navigation-arrow decorations
   render as icons.
3. Open the library and verify its settings trigger renders correctly; hover
   every icon-only control in scope and confirm an explanatory tooltip.
4. Expand and collapse filter sections, then add and remove filters. Confirm
   chevrons and removal icons render while filter text remains readable.
5. Add and remove review/preview tags, confirming their removal controls use
   visible Font Awesome icons and remain keyboard operable.
6. Open a media preview and confirm its existing toolbar icons still render,
   operate, and remain accessible.

## Testing Strategy

The Rust workspace test suite remains the primary automated gate. A narrow
source-level regression test is appropriate because this change is markup and
asset-convention focused: it verifies the local stylesheet, required Font
Awesome classes, removed patterns, and the documented agent rule without
requiring a browser harness. Manual desktop verification confirms that bundled
webfonts load and that CSS layout/focus behavior remains correct in the real
application.

## Performance Considerations

No new runtime dependency or network request is introduced. The app continues
to use its already-bundled Font Awesome stylesheet and webfonts. Replacing text
glyphs with existing icon classes has no meaningful runtime cost.

## Migration Notes

No data, API, command, capability, or persisted-state migration is required.

## References

- Framing: `context/changes/use-fontawesome-icons/frame.md`
- App markup and existing Font Awesome loading: `src/app.rs`
- UI styling: `assets/styles.css`
- Repository implementation rules: `AGENTS.md`

## Progress

### Phase 1: Standardize Scoped Icons and Enforce the Convention

Automated:

- [x] 1.1 Replace scoped icon-only and decorative glyphs with local Font Awesome Free markup.
- [x] 1.2 Add accessibility metadata, styling adjustments, future-agent guidance, and regression coverage.
- [x] 1.3 Run `cargo fmt --check` and `cargo test --workspace`.

Manual:

- [x] 1.4 Verify the refreshed icons, tooltips, keyboard operation, and existing preview toolbar in the desktop app.

# Frame Brief: Font Awesome icon consistency

> Framing step before `/10x-plan`. This document separates the requested
> visual outcome from the implementation rule that will keep it consistent.

## Reported Observation

The app mixes Font Awesome Free, Unicode glyphs, and text for icon-only and
decorative UI elements, producing inconsistent visual language.

## Initial Framing (preserved)

- **User's stated cause or approach**: Change all app icons to Font Awesome icons and make future agent implementations use them too.
- **User's proposed direction**: Apply Font Awesome Free across the app and record a durable agent-facing convention.
- **Pre-dispatch narrowing**: Cover only icon-only controls and decorative marks; retain text-labeled workflow actions.

## Dimension Map

The observation could originate at these dimensions:

1. **Icon asset availability** — Font Awesome Free must be bundled and usable offline.
2. **Existing icon-only/decorative inventory** — Unicode glyphs must be identified without converting meaningful text controls.
3. **Accessibility semantics** — icon-only controls need labels and tooltips after their visible text is removed.
4. **Future-agent convention** — repository guidance must make the library choice explicit for later changes.

## Hypothesis Investigation

| Hypothesis | Evidence | Verdict |
| --- | --- | --- |
| Font Awesome is unavailable or incomplete | `src/app.rs:1707` links the local Font Awesome stylesheet; the preview toolbar already uses `fa-solid` classes. | NONE |
| Unicode glyphs remain in scoped UI | Folder pickers use `⌑` and `→`; settings uses `⚙`; disclosure, tag-remove, and applied-filter controls use `+`, `−`, or `×` (`src/app.rs:1731`, `:1804`, `:1822`, `:1855`, `:2016`, `:2229`). | STRONG |
| Accessibility would regress if text were removed indiscriminately | Many buttons contain user-facing workflow labels such as “Import media”, “Cancel”, and “Start review” (`src/app.rs:1808`, `:1936`, `:2261`). | STRONG |
| Future work lacks a durable icon rule | `AGENTS.md` defines UI/code conventions but contains no icon-library requirement. | STRONG |

## Narrowing Signals

- The requested scope explicitly excludes text-labeled workflow buttons.
- Font Awesome Free is already local and proven in the preview toolbar.

## Cross-System Convention

The repository’s `AGENTS.md` is the established instruction source for coding agents. It is therefore the correct durable location for the icon-system rule, while component-level `aria-label` and `title` attributes retain accessible names for icon-only controls.

## Reframed Problem Statement

> **The actual problem to plan around is**: standardize every icon-only control and decorative mark on locally bundled Font Awesome Free, while preserving text-labeled actions and making that distinction an explicit repository convention for future agents.

This is not a blanket conversion of buttons into icons. The change needs a bounded inventory, accessible replacements, visual verification, and an `AGENTS.md` rule that prevents new Unicode or ad-hoc icon glyphs from reappearing.

## Confidence

- **HIGH** — the local asset is already in use, the remaining glyphs are identifiable in one frontend file, and the requested scope is precise.

## What Changes for `/10x-plan`

Plan one vertical UI slice that replaces the scoped Unicode controls with local Font Awesome Free icons and validates accessible labels/tooltips, plus a repository-guideline update that requires Font Awesome Free for all future icon-only controls and decorative marks.

## References

- Frontend icon inventory: `src/app.rs:1707`, `src/app.rs:1731`, `src/app.rs:1804`, `src/app.rs:1822`, `src/app.rs:1855`, `src/app.rs:2016`, `src/app.rs:2229`
- Existing preview precedent: `src/app.rs:2146`
- Local icon styling: `assets/styles.css:60`, `assets/styles.css:83`
- Future-agent guidance: `AGENTS.md`

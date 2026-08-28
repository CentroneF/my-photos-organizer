# Review and decide media UX changes — Plan Brief

> Full plan: `context/changes/review-and-decide-media-ux-changes/plan.md`

## What & Why

This change turns the post-library-selection experience into a focused, full-window workspace. It removes outdated onboarding chrome, makes the active library and destructive cleanup easier to navigate, and keeps the complete review card visible without scrolling.

The goal is a calmer library-management flow without changing the protected catalogue, explicit import decisions, or source-media safety guarantees.

## Starting Point

The app always renders a blue two-column shell and numbered setup progress, including after a user has selected a library. Library-home controls and inline cleanup sit in `src/app.rs`; media previews use a fixed CSS height and can make review controls fall below the viewport.

## Desired End State

Once a library path is selected, setup, unlock, home, review, and cleanup use a centered single-column workspace with no sidebar or numbered steps. Home opens the active library contents in Finder/Explorer and reaches cleanup through a dedicated page. Every review card fits the complete image/video decision experience in the viewport.

## Key Decisions Made

| Decision | Choice | Why |
| --- | --- | --- |
| Workspace boundary | Begin after any library path is selected | Keeps onboarding available only when it is useful. |
| Step indicators | Remove entirely | The experience no longer represents a two-step flow. |
| Folder action | Open library contents | Matches the requested Finder/Explorer behavior. |
| Folder security | Native no-argument command | Prevents renderer-controlled path launching. |
| Lock controls | Remove everywhere | Manual locking is no longer part of the product UI. |
| Cleanup flow | Dedicated page, then home | Separates a sensitive action while preserving a clear exit. |
| Review sizing | Entire card fits viewport | Tags and explicit decisions remain continuously accessible. |
| Content alignment | Centered | Preserves a focused, familiar workspace on wide displays. |

## Scope

**In scope:** selected-library layout; stepper removal; native active-library folder opening; home action rearrangement; removal of manual lock UI; dedicated danger page; viewport-bounded review card.

**Out of scope:** security-model changes, filesystem permissions for arbitrary paths, changes to cleanup semantics, media-processing behavior, and duplicate/search functionality.

## Architecture / Approach

Dioxus continues to own view state through `step`; a layout modifier separates onboarding from selected-library states. Tauri remains responsible for opening the active library folder, resolving its path server-side before using the installed opener plugin. CSS uses viewport-aware constraints to divide the review card's available height between metadata/controls and media.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Selected-library workspace | Full-window centered views without sidebar or stepper | Preserving onboarding for pre-selection states. |
| 2. Home actions and cleanup | Safe folder opener, dedicated danger page, no lock UI | Never opening an arbitrary path or weakening cleanup safeguards. |
| 3. Viewport-bounded review | Full photo/video decision card without scrolling | Keeping all controls reachable across screen sizes. |

**Prerequisites:** Existing protected-library and review flows remain functional; macOS and Windows desktop smoke-test access is available.
**Estimated effort:** ~2–3 focused sessions across three vertical phases.

## Open Risks & Assumptions

- OS-folder opening is inherently platform-integrated and needs manual Finder/Explorer verification; native tests cover only path resolution/error handling.
- Very small or accessibility-scaled windows may need a compact layout fallback to keep every decision control visible.
- Retaining the native lock command does not expose a manual UI path; it preserves the existing session-security primitive for internal use.

## Success Criteria (Summary)

- Selected-library views are centered, full-window, and free of the blue sidebar and numbered onboarding indicators.
- Users can open only the active library's contents and safely enter/leave the dedicated cleanup page.
- At minimum window size, image and video review cards expose preview, tags, date, Skip, and Import without page scrolling.

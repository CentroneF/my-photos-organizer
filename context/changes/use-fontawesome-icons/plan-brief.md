# Plan Brief: Use Font Awesome Icons

## Outcome

Standardize the app's icon-only controls and decorative UI marks on the
already-bundled Font Awesome Free assets. Keep text-labelled workflow actions
as text.

## Decisions

- Scope covers icon-only controls and decorative marks only.
- Filter-chip labels remain text; their trailing removal mark becomes a Font
  Awesome icon.
- Every icon-only interactive control receives both an accessible name and a
  visible tooltip.
- The convention is enforced in both `AGENTS.md` and lightweight source-level
  regression coverage.

## One vertical phase

Replace the remaining scoped Unicode glyphs in the Dioxus UI, align their
styling with the existing Font Awesome preview toolbar, add the durable agent
rule and regression checks, then verify in the desktop app.

## Guardrails

- Reuse the bundled local Font Awesome Free stylesheet and webfonts.
- Do not add a CDN, an icon package, or a new runtime dependency.
- Preserve text labels and existing interaction behavior.
- Do not convert ordinary text or media metadata symbols that are outside the
  agreed icon-only/decorative scope.

See [plan.md](plan.md) for the implementation and verification detail.

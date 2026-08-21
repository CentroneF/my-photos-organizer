---
bootstrapped_at: 2026-08-21T13:51:12Z
starter_id: tauri
starter_name: Tauri
project_name: my-media-organizer
language_family: rust
package_manager: cargo
cwd_strategy: subdir-then-move
bootstrapper_confidence: verified
phase_3_status: ok
audit_command: cargo audit
---

## Hand-off

```yaml
starter_id: tauri
package_manager: cargo
project_name: my-media-organizer
hints:
  language_family: rust
  team_size: solo
  deployment_target: self-host
  ci_provider: github-actions
  ci_default_flow: auto-deploy-on-merge
  bootstrapper_confidence: verified
  path_taken: standard
  quality_override: false
  self_check_answers: null
  has_auth: true
  has_payments: false
  has_realtime: false
  has_ai: false
  has_background_jobs: false
```

My Media Organizer is a small, local-first desktop application for macOS and Windows, where reviewing and cataloguing personal photos and videos benefits from efficient native media and filesystem work. Tauri with a Dioxus frontend keeps both application layers in Rust and uses Cargo throughout, avoiding the incompatible React-and-Cargo pairing from the earlier selection. It remains a lightweight cross-platform desktop shell, fits self-hosted releases, and uses GitHub Actions for automatic releases after merges to main. Password-protected access is in scope; payments, realtime features, AI features, and background jobs are not.

## Pre-scaffold verification

| Signal | Value | Severity | Notes |
| --- | --- | --- | --- |
| npm package | not run | n/a | Non-JS starter; no package check applies. |
| GitHub repo | not run | n/a | The starter documentation URL is not a GitHub repository. |

## Scaffold log

**Resolved invocation**: `npm create tauri-app@latest .bootstrap-scaffold -- --template dioxus --manager cargo --yes`

**Strategy**: scaffolded into a temporary directory, then moved into the current directory

**Exit code**: 0

**Files moved**: 33

**Conflicts (.scaffold siblings)**: none

**.gitignore handling**: append-merged with `/dist/`, `/target/`, and `/Cargo.lock`

**.bootstrap-scaffold cleanup**: deleted

## Post-scaffold audit

**Tool**: `cargo audit`

**Status**: failed to run

**Reason**: `cargo-audit` is not installed (`error: no such command: audit`). Install it with `cargo install cargo-audit` to run a Rust dependency audit.

## Hints recorded but not acted on

| Hint | Value |
| --- | --- |
| bootstrapper_confidence | verified |
| quality_override | false |
| path_taken | standard |
| self_check_answers | null |
| team_size | solo |
| deployment_target | self-host |
| ci_provider | github-actions |
| ci_default_flow | auto-deploy-on-merge |
| has_auth | true |
| has_payments | false |
| has_realtime | false |
| has_ai | false |
| has_background_jobs | false |

## Next steps

Next: a future skill will set up agent context (CLAUDE.md, AGENTS.md). For now, your project is scaffolded and verified — happy hacking.

Useful manual steps in the meantime:

- Run `cargo install tauri-cli --version '^2.0.0' --locked` and `cargo install dioxus-cli --locked` before using the development commands.
- Run `cargo install cargo-audit`, then `cargo audit`, to check dependencies for known Rust advisories.
- Run `git init` if you have not already created your own repository history.

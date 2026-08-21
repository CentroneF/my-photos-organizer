---
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
---

## Why this stack

My Media Organizer is a small, local-first desktop application for macOS and Windows, where reviewing and cataloguing personal photos and videos benefits from efficient native media and filesystem work. Tauri provides a lightweight cross-platform desktop shell with Rust for that work and a web-based interface, while keeping the product independent of a hosted deployment platform. It is the curated default for Rust desktop projects, passes all four agent-friendly criteria, and has verified bootstrap support. Self-hosted releases fit the distribution choice, with GitHub Actions automatically publishing after merges to main. Password-protected access is in scope; payments, realtime features, AI features, and background jobs are not.

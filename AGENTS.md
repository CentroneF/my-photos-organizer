# Repository Guidelines

Photo Handler is a local-first macOS and Windows desktop application built with Tauri and Dioxus. The workspace has a Dioxus web frontend plus a native Tauri crate; use the existing starter wiring as the reference until a feature establishes a new pattern.

## Non-negotiables

- Never delete or move a user's original media without explicit user authorization; the product guardrail is recorded in @context/foundation/prd.md.
- Plan and implement changes as vertical, end-to-end slices that can be manually verified through the frontend; do not deliver horizontal-only phases. See @context/foundation/lessons.md.
- Create a branch named for each change before implementation. After a plan is complete, ask for a commit before suggesting implementation. Confirm every commit message with the user before committing.
- Keep `Cargo.lock` committed. Do not add generated `target/`, `dist/`, `.vscode/`, or local `.env*` files; @.gitignore allows `.env.example`.

## Project Structure

- `src/` contains the Dioxus application. Keep the root component and UI event handling in @src/app.rs; assets are declared from `assets/`.
- `src-tauri/` contains the native Tauri application. Add Rust commands in @src-tauri/src/lib.rs and register each one through the existing `invoke_handler` before calling it from the UI.
- @Cargo.toml is the workspace manifest and @src-tauri/Cargo.toml owns native dependencies. Keep shared dependency versions pinned through `Cargo.lock`.
- @context/foundation/ holds the PRD, stack selection, lessons, and future roadmap; treat these as product and workflow sources of truth.

## Development and Verification

- Run `cargo tauri dev` from the repository root to build and open the desktop application.
- Run `cargo tauri build` to produce a release bundle.
- Run `cargo test --workspace` after adding or changing Rust behavior. No test framework, test files, or CI workflow are configured yet; add them with the feature that first needs them.

## Code and Git Conventions

- Follow the current Rust structure: Dioxus components return `Element`; UI-to-native calls use Tauri command names and serializable argument structs as shown in @src/app.rs.
- Keep public native command names and their frontend invocation strings synchronized. Change capability configuration alongside any command whose permissions require it; see @src-tauri/capabilities/default.json.
- Recent commits use Conventional Commit-style prefixes such as `feat:`, `docs:`, and `chore:`. Keep commit subjects concise and imperative.

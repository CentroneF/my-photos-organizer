# Protected library setup — Implementation Plan

## Overview

Create the first user-visible Photo Handler flow: choose a fixed, empty managed-library folder, create an encrypted local catalogue protected by a password and local recovery answer, and reopen that same library safely. The flow must never move, delete, or overwrite user media or arbitrary existing folders.

## Current State Analysis

The app is the unmodified Dioxus/Tauri starter: a single greeting form in `src/app.rs` invokes one native command in `src-tauri/src/lib.rs`. No persistence, native dialog, password, filesystem-validation, database, or test support exists. The PRD requires a user-selected fixed library location that can reopen existing state, and the first roadmap slice explicitly makes that a password-protected trust boundary.

## Desired End State

On first launch, a person can select an empty writable folder, define and confirm a password plus a custom local recovery question/answer, and receive a clear success state. The app creates only a dedicated `.photo-handler/` state directory containing an encrypted, versioned SQLite catalogue; it leaves all other folder contents and all media untouched. On later launches, it reads the remembered path, validates its own marker, unlocks the catalogue with the password, and can reset the password after a correct recovery answer.

### Key Discoveries

- `context/foundation/prd.md:60` requires a fixed user-chosen library folder that can load existing application data; `:82` requires all catalogue data to stay local.
- `context/foundation/prd.md:93` requires password protection, while `:97` prohibits deleting originals without explicit authorization.
- `src-tauri/src/lib.rs:2` exposes the native-command seam; `src/app.rs:11` already demonstrates frontend-to-native serialization through Tauri invoke.
- `src-tauri/Cargo.toml:20` has no persistence or security dependencies yet, and `src-tauri/capabilities/default.json:5` has no dialog permission.
- SQLite is a portable, transactional application-file format; use `rusqlite` with the bundled SQLCipher feature rather than a key-value store or JSON for the extensible media catalogue. [SQLite application-file guidance](https://www.sqlite.org/appfileformat.html), [rusqlite features](https://docs.rs/crate/rusqlite/latest)

## What We're NOT Doing

- Importing, copying, moving, deleting, encrypting, or scanning media files.
- Adding media, tag, hash, duplicate, or search tables beyond the minimal library identity and migration state.
- Adding cloud sync, email recovery, OS-keychain sessions, multi-user access, or shared libraries.
- Treating the recovery question as high-assurance authentication; it is an explicitly accepted local-only convenience tradeoff.
- Accepting a non-empty folder, an arbitrary SQLite file, or an unrecognized application-state directory as a new library.

## Implementation Approach

Use a dedicated native library-management module as the authority for all filesystem and catalogue operations. The Dioxus UI collects user input and renders serializable command results, while Rust validates a selected folder before mutating it. On setup, generate a random database key, encrypt the SQLite catalogue with SQLCipher, and store password- and recovery-answer-derived wraps of that key using Argon2id. Persist only the selected-library path in app-local settings; the selected folder itself remains self-contained through `.photo-handler/` marker and version metadata.

## Critical Implementation Details

The password cannot directly be the database key if the local recovery answer can reset it. Generate a random database key and wrap it independently using Argon2id-derived material from the password and recovery answer; keep no plaintext password, answer, or unwrapped key after the active command/session. Validate all setup prerequisites before creating `.photo-handler/`; if initialization fails, leave arbitrary user files untouched and remove only state created by that failed attempt when it is safe to do so.

## Phase 1: Create a protected managed library

### Overview

Deliver a complete first-run setup flow that lets a user select an empty folder, protect a new local catalogue, and see that the fixed library was created without touching their media.

### Changes Required

#### 1. Native dependencies, capability, and library manager

**Files**: `src-tauri/Cargo.toml`, `src-tauri/capabilities/default.json`, `src-tauri/src/lib.rs`, new native library-management modules under `src-tauri/src/`

**Intent**: Add the native dependencies and command surface required to select a directory, create a SQLCipher-backed SQLite catalogue, derive keys with Argon2id, generate cryptographically random material, and persist the selected-library pointer. Keep all filesystem mutation and security-sensitive work in Rust.

**Contract**: Expose serializable setup request/result/error DTOs and register a command that validates an empty, writable folder before creating `.photo-handler/`. Create a marker/version record and initial transactional schema. Persist a random DB key only as two Argon2id-protected wraps—one for the password and one for the custom recovery answer—and never persist plaintext credentials. Add the least dialog capability required by the chosen native picker integration.

#### 2. First-run setup screen

**File**: `src/app.rs`

**Intent**: Replace the starter greeting UI with a setup screen that guides a person through choosing a managed-library folder, setting and confirming a password, and defining a custom recovery question/answer.

**Contract**: Invoke the native directory picker and setup command using serializable payloads. Render loading, validation-error, and success states without frontend `unwrap` failures. Success identifies the selected folder and states that only Photo Handler state was created; the UI must not imply media was copied, encrypted, or modified.

#### 3. Setup tests and verification assets

**Files**: native module test files under `src-tauri/src/`, optionally `src-tauri/Cargo.toml`

**Intent**: Make safety-sensitive setup behavior repeatable without requiring a live desktop window.

**Contract**: Use temporary directories to cover successful initialization, rejected non-empty folder, marker creation, idempotent protection against duplicate initialization, and failure paths that leave no new application state in an invalid target. Assert that setup never deletes or modifies pre-existing files.

### Success Criteria

#### Automated Verification

- `cargo test --workspace` passes, including temporary-directory setup safety tests.
- `cargo check --workspace` succeeds.
- A code search confirms no setup path deletes, moves, or overwrites arbitrary selected-folder content.

#### Manual Verification

- In the desktop UI, a user can select an empty folder, enter matching password and recovery fields, and see successful protected-library creation.
- The selected folder contains only the documented `.photo-handler/` application state and no media operation occurred.
- Selecting a non-empty or inaccessible folder produces an error without changing it.

**Implementation Note**: After completing this phase and all automated verification passes, pause for manual confirmation before proceeding.

---

## Phase 2: Reopen, unlock, and recover the library

### Overview

Deliver the subsequent-launch experience: reopen the remembered protected library, unlock it with the password, and reset that password through the explicitly accepted local recovery question flow.

### Changes Required

#### 1. Reopen, unlock, and recovery operations

**Files**: native library-management modules under `src-tauri/src/`, `src-tauri/src/lib.rs`

**Intent**: Make the selected library reliably discoverable on restart and ensure only a recognized, compatible Photo Handler library can be opened.

**Contract**: Read the app-local path pointer, validate the `.photo-handler/` marker and schema version, unlock SQLCipher only after a correct password-derived key unwrap, and expose a safe “open existing library” path when the pointer is missing or stale. A correct recovery answer may re-wrap the existing random database key with a new confirmed password; wrong credentials, malformed markers, unsupported schema versions, or unavailable folders must return clear errors without altering user data.

#### 2. Unlock and recovery UI

**File**: `src/app.rs`

**Intent**: Give a returning user a focused unlock screen and a local recovery path without suggesting any cloud/email reset capability.

**Contract**: Render remembered-library status, password entry, generic incorrect-password feedback, stale-path handling, an explicit “open existing library” action, and a recovery flow that presents the custom question only after the user chooses recovery. Require new-password confirmation after successful answer validation and erase sensitive form signals after completion or cancellation.

#### 3. Reopen/recovery tests and lifecycle checks

**Files**: native module test files under `src-tauri/src/`, optionally `src/app.rs`

**Intent**: Cover the trust-boundary lifecycle beyond the initial happy path.

**Contract**: Test reopen with the correct password, wrong password rejection, recovery-answer reset, wrong-answer rejection, stale pointer handling, malformed/foreign state rejection, and migration-version validation. Verify every rejected case leaves the catalogue and surrounding user files unchanged.

### Success Criteria

#### Automated Verification

- `cargo test --workspace` passes, including reopen, wrong-password, recovery, and malformed-state tests.
- `cargo check --workspace` succeeds.
- Test assertions verify rejected unlock/recovery paths preserve catalogue and user-folder contents.

#### Manual Verification

- After closing and relaunching, the app recognizes the remembered library and unlocks it with the password.
- A wrong password is rejected without corrupting the library.
- The recovery question can reset the password locally; the new password unlocks the same library afterward.
- A moved/unavailable folder can be selected again only when its Photo Handler marker and password are valid.

**Implementation Note**: After completing this phase and all automated verification passes, pause for manual confirmation before considering the change complete.

## Testing Strategy

### Unit Tests

- Folder validation accepts only empty writable folders for initialization.
- Setup creates the expected marker, schema version, and encrypted catalogue while leaving unrelated files untouched.
- Password and recovery-answer key wraps unlock the same database key only with correct credentials.
- Reopen and migration checks reject malformed, foreign, or unsupported state without mutation.

### Integration Tests

- Native command DTOs return stable success and error results for setup, unlock, recovery, and path selection.
- The app-local pointer can reopen a valid library and safely handles a stale/moved path.

### Manual Testing Steps

1. Create a new empty temporary folder, initialize a protected library, and inspect that only `.photo-handler/` was created.
2. Restart the app and unlock the same library using the original password.
3. Enter a wrong password and confirm no state changes; then use the custom recovery question to set a new password and unlock again.
4. Try a non-empty folder, a folder without permissions, and a foreign/malformed `.photo-handler/` folder; confirm each fails safely.

## Performance Considerations

Argon2id deliberately makes setup, unlock, and recovery moderately expensive; run each once per explicit user action and keep the derived database key only for the active session. The initial schema is intentionally tiny, so SQLite operations are not a catalogue-performance concern in this slice.

## Migration Notes

The initial database schema must include a version record and apply migrations transactionally. Existing folders are never converted automatically: only folders carrying the expected marker and compatible schema can be reopened. Password reset re-wraps the existing random database key; it must not create a new catalogue or rewrite media.

## References

- Product requirement: `context/foundation/prd.md:60`
- Product guardrail: `context/foundation/prd.md:97`
- Roadmap slice: `context/foundation/roadmap.md:64`
- Native command entry point: `src-tauri/src/lib.rs:2`
- Frontend invocation pattern: `src/app.rs:11`
- SQLite application-file guidance: https://www.sqlite.org/appfileformat.html
- SQLite transactions: https://www.sqlite.org/transactional.html
- rusqlite features: https://docs.rs/crate/rusqlite/latest
- Security-question risk: https://cheatsheetseries.owasp.org/cheatsheets/Choosing_and_Using_Security_Questions_Cheat_Sheet.html

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles.

### Phase 1: Create a protected managed library

#### Automated

- [x] 1.1 Add native protected-library setup, encrypted catalogue initialization, and safety tests — a26991a
- [x] 1.2 Add the first-run folder, password, and recovery setup screen — a26991a
- [x] 1.3 Verify workspace tests, compilation, and setup no-mutation guarantees — a26991a

#### Manual

- [x] 1.4 Confirm protected-library creation and safe rejected-folder behavior in the desktop UI — a26991a

### Phase 2: Reopen, unlock, and recover the library

#### Automated

- [x] 2.1 Add native reopen, unlock, recovery, and lifecycle safety behavior with tests — 3c86ab1
- [x] 2.2 Add returning-user unlock, recovery, and existing-library UI states — 3c86ab1
- [x] 2.3 Verify workspace tests, compilation, and rejected-path preservation guarantees — 3c86ab1

#### Manual

- [x] 2.4 Confirm restart unlock, local recovery, and safe stale/foreign-library behavior in the desktop UI — 3c86ab1

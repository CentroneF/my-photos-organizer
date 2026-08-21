# 10x-new branch setup — Implementation Plan

## Overview

Update the 10x-new skill so a newly initialized change has a dedicated branch created from the latest `origin/main` and correctly tracks a same-named remote branch. The workflow must avoid the current `origin/main` upstream mismatch, and it must not create a change record when branch setup fails.

## Current State Analysis

10x-new validates the change ID and change/archive folder collisions, then immediately creates `context/changes/<change-id>/change.md`. It contains no branch, remote, or upstream handling.

The current repository rule requires every 10x-new change to have a same-named branch based on `origin/main`. Creating a branch directly from a remote-tracking start point normally makes that start point the upstream, which is why a feature branch can incorrectly track `origin/main`.

## Desired End State

Running 10x-new for a new ID fetches the latest `origin/main`, creates and checks out a same-named local branch based on it without tracking `origin/main`, publishes that branch, and verifies that it tracks `origin/<change-id>`. Only then does it create the change folder and `change.md`.

If the branch is already checked out, the skill makes no Git changes but continues the normal change-record flow. If setup cannot proceed, it preserves the working tree and asks the user to confirm a verified alternate ID such as `<change-id>-2` before retrying.

### Key Discoveries

- `.agents/skills/10x-new/SKILL.md:58-68` performs only ID and folder validation; creation begins at `:70`.
- `context/foundation/lessons.md:12-24` requires 10x-new to create a branch named after the change ID over `origin/main`.
- A branch created from `origin/main` must use `--no-track`; publishing with `git push -u origin <change-id>` is what establishes the intended `origin/<change-id>` upstream.
- `.agents/.10x-cli-manifest.json` records installed skill hashes and is not an authored behavior specification; it must not be manually updated for this local skill change.

## What We're NOT Doing

- Changing implicit change-folder creation in 10x-plan, 10x-research, or 10x-frame.
- Reusing, resetting, or moving an existing local or remote branch.
- Automatically accepting an alternate ID after a branch conflict.
- Editing the installed-skill manifest or adding a new test harness.

## Implementation Approach

Make the 10x-new workflow explicit about Git state before its existing creation stage. Preserve its existing ID and folder validation, add a branch preflight and a dedicated branch-setup stage, then gate folder creation on successful branch verification. Treat the currently checked-out matching branch as the sole no-op exception; all other local or remote name collisions require an interactive confirmation of a verified alternate ID.

## Critical Implementation Details

The sequence is load-bearing: fetch first, create from the fetched `origin/main` with no upstream, then push with `-u` to establish `origin/<change-id>`. Do not set an upstream before the remote branch exists, and do not let any failure after validation create `change.md`.

## Phase 1: Define safe branch preflight and conflict handling

### Overview

Extend the skill contract with the exact Git checks, no-op behavior, and user-confirmed alternate-name path needed before any branch or folder mutation.

### Changes Required

#### 1. Branch validation and setup sections

**File**: `.agents/skills/10x-new/SKILL.md`

**Intent**: Add a branch preflight after the existing structural validation and before creation. It must distinguish the branch already being checked out from every other collision, and it must make a failed branch operation leave no change folder behind.

**Contract**: Specify all of the following in the skill:

- If the current branch name equals `<change-id>`, skip fetch, switching, creation, publication, and upstream changes; continue with the existing change-folder workflow.
- Otherwise fetch `origin/main`; abort cleanly if the fetch fails.
- Check both the local and remote branch namespaces for `<change-id>` before creation. A collision must stop and offer the first suffix starting at `<change-id>-2` that is verified free in both namespaces.
- Ask the user to confirm the proposed alternate ID rather than selecting it automatically; on confirmation, restart validation with that ID.
- Create from the freshly fetched `origin/main` without tracking it, publish the branch, and verify that the checked-out branch is `<change-id>` and its upstream is `origin/<change-id>`.
- Never reset, move, or reuse an existing branch. Any Git failure after the no-op check stops before the folder-creation stage.

### Success Criteria

#### Automated Verification

- The skill text includes the fetch, no-track creation, same-name publish/upstream, and local/remote collision checks.
- The skill text explicitly prohibits `origin/main` as the feature branch's upstream and prohibits folder creation after branch-setup failure.

#### Manual Verification

- A reviewer can follow the documented flow and see that an already-checked-out matching branch triggers no Git mutation.
- A reviewer can follow the documented collision flow and see that an alternate candidate requires explicit user confirmation.

**Implementation Note**: After completing the textual checks, pause for human confirmation of the documented behavior before proceeding to the next phase.

---

## Phase 2: Gate change creation and update the handoff

### Overview

Move the existing folder/file creation semantics after verified branch setup and make the result visible in the skill's success output.

### Changes Required

#### 1. Creation ordering and success output

**File**: `.agents/skills/10x-new/SKILL.md`

**Intent**: Preserve the current title, notes, and change.md rules, but make them conditional on the Git branch contract from Phase 1. Tell users both what was created and which remote branch is now tracked.

**Contract**: Renumber the creation procedure as needed so the directory and `change.md` are created only after successful branch setup or the documented already-checked-out no-op. Update the successful output to identify the checked-out local branch and `origin/<change-id>` upstream, while retaining the existing next-step and clipboard behavior.

#### 2. Verification guidance

**File**: `.agents/skills/10x-new/SKILL.md`

**Intent**: Add concise verification guidance for the normal path and failure paths so future agents do not reintroduce the upstream mismatch.

**Contract**: Include a disposable-repository verification matrix covering: fresh ID, current matching branch, local collision, remote collision, fetch failure, branch creation failure, and push failure. Every failed setup case asserts that no new change folder is written.

### Success Criteria

#### Automated Verification

- The updated skill has a single ordered workflow in which successful branch verification precedes folder creation.
- The success output includes both `<change-id>` and `origin/<change-id>`.
- Existing `change.md` shape, title derivation, notes behavior, and next-step recommendation remain represented in the skill.

#### Manual Verification

- In a disposable repository with an `origin` remote, a fresh valid ID produces a local branch based on fetched main, a same-named remote branch, correct upstream tracking, and a new change.md.
- The same disposable setup confirms that fetch, branch-creation, and push failures leave no newly created change folder.

**Implementation Note**: After automated checks pass, pause for the human to confirm the disposable-repository scenarios before considering the change complete.

## Testing Strategy

### Documentation Checks

- Verify the skill has one unambiguous order: validate → branch preflight/setup → create change record → handoff.
- Search the skill for every Git command and confirm no path tracks `origin/main` as the feature branch upstream.
- Verify the existing change.md schema and clipboard handoff instructions are retained.

### Manual Testing Steps

1. In a disposable repository with a bare `origin`, run the documented fresh-ID path and inspect the current branch and upstream.
2. Repeat while already on the requested branch and confirm Git state is unchanged while normal folder logic remains available.
3. Create matching local and remote branches separately; confirm each produces a confirmation prompt for a verified `-2` candidate and does not write the requested folder.
4. Simulate fetch, branch-creation, and push failures; confirm each stops before change-folder creation.

## Performance Considerations

The workflow adds one fetch and remote-name lookup only when not already on the requested branch. The already-checked-out path intentionally avoids all network and Git mutations.

## Migration Notes

Existing change folders and branches are untouched. The new behavior applies only to later 10x-new invocations.

## References

- `.agents/skills/10x-new/SKILL.md:58-136`
- `context/foundation/lessons.md:12-24`
- `.agents/skills/10x-new/references/change-md.md`

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Define safe branch preflight and conflict handling

#### Automated

- [x] 1.1 Verify branch preflight, no-track setup, collision detection, and failure gating are documented
- [x] 1.2 Verify the skill forbids the incorrect origin/main upstream and auto-selection of alternates

#### Manual

- [ ] 1.3 Confirm the already-checked-out and alternate-ID interaction contracts

### Phase 2: Gate change creation and update the handoff

#### Automated

- [ ] 2.1 Verify branch setup precedes change-folder creation and success output reports correct tracking
- [ ] 2.2 Verify existing change-record and handoff behavior remains documented

#### Manual

- [ ] 2.3 Confirm fresh, collision, and Git-failure paths in a disposable repository

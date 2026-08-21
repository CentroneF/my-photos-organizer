# 10x-new branch setup — Plan Brief

> Full plan: `context/changes/10x-new-skill-update/plan.md`

## What & Why

10x-new will create each new change on a matching Git branch based on the latest `origin/main`, then publish it with `origin/<change-id>` as its upstream. This prevents the current behavior where a feature branch may incorrectly track `origin/main` and reject a normal push.

## Starting Point

10x-new currently validates IDs and folders, then writes `change.md`; it has no Git workflow. The repository lessons require a same-named branch over `origin/main`, but the previous direct creation approach selected the wrong upstream.

## Desired End State

A fresh change ID results in a fetched mainline, a checked-out same-named branch, a same-named remote branch, correct upstream tracking, and then the change record. If any Git step fails, no new change folder exists; branch-name conflicts instead require the user to confirm a verified alternative such as `change-id-2`.

## Key Decisions Made

| Decision | Choice | Why |
|---|---|---|
| Already checked out | Skip only Git operations; continue normal folder flow | Avoids needless Git mutation without suppressing 10x-new’s core change-record behavior. |
| Upstream setup | Publish immediately with `git push -u origin <change-id>` | A same-named remote branch must exist before it can be the upstream. |
| Name collision | Propose a verified suffix and ask for confirmation | Prevents accidental branch reuse or an unannounced change-ID rewrite. |
| Scope | Update only 10x-new | Keeps the requested correction contained; implicit creation paths remain unchanged. |

## Scope

**In scope:**

- Git preflight, branch creation, publication, upstream verification, and conflict behavior in 10x-new.
- Ordering and output changes that gate `change.md` creation on successful branch setup.
- Documented disposable-repository verification scenarios.

**Out of scope:**

- Other skills that can implicitly create change folders.
- Existing branches or folders.
- Changes to the installed-skill manifest or a new test harness.

## Architecture / Approach

The skill will perform structural validation, then Git preflight. A matching current branch takes the no-op branch path; all other paths fetch main, prove the requested name is unused, create the branch with no upstream, publish it with the correct same-name upstream, verify state, and only then create the change record.

## Phases at a Glance

| Phase | What it delivers | Key risk |
|---|---|---|
| 1. Safe branch preflight | No-op, fetch, collision, confirmation, and branch rules | Accidentally tracking `origin/main` or reusing a branch |
| 2. Creation gate and handoff | Folder ordering, output, and verification guidance | Writing a change record after a partial Git failure |

**Prerequisites:** Git repository with access to `origin`; a disposable repository for manual verification.

## Open Risks & Assumptions

- Immediate publishing intentionally creates a remote branch before a first commit; this is required by the requested upstream contract.
- Remote branch lookup must distinguish a genuine absence from an unavailable remote, and abort safely in the latter case.

## Success Criteria (Summary)

- A fresh ID produces correct local/remote branch tracking before `change.md` is created.
- An already-checked-out requested branch undergoes no Git mutation.
- Conflicts and Git failures do not create a change folder, and conflicts ask before switching to an alternate ID.

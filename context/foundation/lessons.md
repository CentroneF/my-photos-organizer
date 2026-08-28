# Lessons Learned

> Append-only register of recurring rules and patterns. Re-read at start by /10x-frame, /10x-research, /10x-plan, /10x-plan-review, /10x-implement, /10x-impl-review.

## Plan Vertical End-to-End Phases

- **Context**: plan a change
- **Problem**: every time I plan a change, the phases are horizontal, not vertical on the full stack so im not able to manually verify the changes from the frontend
- **Rule**: Plan vertical, end-to-end phases so each phase is manually verifiable from the frontend.
- **Applies to**: plan

## Create a Branch for Every New Change

- **Context**: When working on a change implementation
- **Problem**: The changes are committed to main.
- **Rule**: When using `/10x-new`, a new branch should be checked out named like the change ID.
- **Applies to**: new

## Create a branch for every new change

- **Context**: Every time a new change is initialized
- **Problem**: No new branch is created.
- **Rule**: When running `/10x-new`, create a new branch over `origin/main` with the name of the change ID.
- **Applies to**: 10x-new

## Commit the plan before implementation

- **Context**: When 10x-plan finishes
- **Problem**: The skill suggests implementing phase 1 instead of committing the plan.
- **Rule**: After `/10x-plan` is completed, ask the user to commit before suggesting `/10x-implement <change-id> phase 1`.
- **Applies to**: plan, plan-review

## Confirm commit messages before committing

- **Context**: Every time a new commit message is generated
- **Problem**: The agent doesn't ask for confirmation.
- **Rule**: Ask for confirmation of the commit message before committing.
- **Applies to**: all

## Push Every Commit

- **Context**: Every time you commit something.
- **Problem**: I might lose progress.
- **Rule**: Every time you commit, push.
- **Applies to**: N/A

# Scripts

## Community Sync

Scripts to help sync community commits into the enterprise repository.
All scripts include `--help` for usage details.

This document illustrates the ParadeDB community sync process that keeps the enterprise repository synchronized with community updates.

## Workflow Diagram

```mermaid
graph TD
    A[Community Repo<br/>paradedb/paradedb] -->|New commits| B[Hourly GitHub Action]
    B --> C{Conflicts?}
    C -->|No| D[Create history tag<br/>community-sync-history-YYYY-MM-DD-HHMMSS]
    C -->|Yes| E[GitHub Action fails<br/>Manual resolution needed]

    D --> F[Create patch branch<br/>enterprise-patch-YYYY-MM-DD-HHMMSS<br/>from origin/main]
    F --> G[Apply community commits<br/>on patch branch]
    G --> H[Push patch branch<br/>to trigger CI]
    H --> I{All CI<br/>passed?}
    I -->|Yes| J[Auto-merge patch to origin/main<br/>& cleanup branch]
    I -->|No| K[Preserve patch branch<br/>for investigation]
    J --> L[origin/main updated<br/>local main untouched]
    K --> M[origin/main stays untouched<br/>local main untouched]

    E --> N[Engineer runs rebase-community-batch.sh<br/>locally from main branch]
    N --> P[Create patch branch from origin/main<br/>& encounter conflicts]
    P --> Q[Resolve conflicts on patch branch<br/>git add . && git rebase --continue]
    Q --> R[Push resolved patch branch]

    R --> S["⚠️ Manual Promotion Required<br/>⚠️ Use promote-branch-manually.sh<br/>or GitHub Actions workflow"]
    S --> T{CI passed?}
    T -->|Yes| U[Promote patch branch to origin/main<br/>local main untouched]
    T -->|No| V[Fix issues & push again]
    V --> S
    U --> L

    style A fill:#e1f5fe
    style I fill:#c8e6c9
    style E fill:#ffecb3
    style M fill:#ffebee
    style T fill:#c8e6c9
    style U fill:#f3e5f5
    style S fill:#fff3cd,stroke:#ff6b6b,stroke-width:4px,color:#000
```

## Scripts Overview

### `rebase-community-batch.sh`

Entry point for community sync operations.

- ⚠️ **This script is automatically run by GitHub Actions every hour** - Only run locally if GitHub Actions fails and prompts you to do so!
- **Local main branch is never touched or modified** - All work happens on patch branches created from `origin/main`
- Creates patch branch from `origin/main` (not local main)
- Applies multiple community commits in batch on the patch branch
- Creates tags anytime it is running a rebase. This helps preserving git history in case manual review is needed
- **Automated CI Validation**: Pushes patch branch to trigger native GitHub CI, polls for completion, and auto-merges to `origin/main` when CI passes
- **Conflict Resolution**: Leaves patch branch with conflicts for manual resolution when needed
- After successful completion, you'll be on a detached HEAD at `origin/main`. Run `git checkout main && git pull` to sync your local main
- Use `--dry-run` to preview changes

```text
Usage: ./scripts/rebase-community-batch.sh [OPTIONS]

Community rebase script that applies multiple commits with CI validation.

Options:
  --max-commits N       Maximum number of commits to process (default: 5)
  --skip-validation     Skip CI validation step
  --dry-run            Show what would be done without making changes
  --debug              Enable debug logging
  -h, --help           Show this help message

Examples:
  ./scripts/rebase-community-batch.sh                              # Apply up to 5 commits
  ./scripts/rebase-community-batch.sh --max-commits 5              # Apply up to 5 commits
  ./scripts/rebase-community-batch.sh --dry-run --max-commits 3    # Preview up to 3 commits
  ./scripts/rebase-community-batch.sh --skip-validation            # Skip CI validation
```

## Normal Operations

Community sync operations are **automated via GitHub Actions** (`.github/workflows/community-rebase.yml`), which runs every hour.

**Manual execution is only needed if the GitHub Action fails** and explicitly asks you to resolve conflicts locally. In that case, see the [Conflict Resolution Workflow](#conflict-resolution-workflow) section below.

## Conflict Resolution Workflow

If a conflict occurs during the GitHub Action, it will preserve the patch branch and ask you to resolve conflicts manually:

```bash
# Run the batch script manually (must be on main branch to start)
# The script will create a patch branch from origin/main (not your local main)
./scripts/rebase-community-batch.sh
```

This will create a new patch branch from `origin/main` and encounter the same conflict. You'll be left on the patch branch with conflicts to resolve:

```bash
# Stage resolved files
git add .

# Continue the rebase
git rebase --continue

# Push your resolved patch branch
git push
```

Now you have the resolved commits on the patch branch. To promote this branch to main, use the **manual promotion workflow**:

### `promote-branch-manually.sh`

Branch promotion workflow for manual conflict resolution and enterprise patch management.

- ⚠️ **This script should ONLY be run via GitHub Actions workflow** - Do not run locally!
- Automatically triggered by the "Promote Branch to Main" workflow in GitHub Actions
- **Use Cases**:
  - Promote patch branches after manual conflict resolution
  - Promote enterprise patches that were squashed/split locally
  - Manual promotion when automated workflow fails

## Enterprise Patch Management

If you need to squash or split enterprise patches (commits after the common ancestor with community main), perform the operations locally and use the promote-branch workflow to merge to main:

```bash
# 1. Make your changes locally on main branch
# Examples:
#   - Squash commits: git rebase -i <community-base-commit>
#   - Split commits: git rebase -i <community-base-commit> and use 'edit'
#   - Amend commits: git commit --amend

# 2. Once your local main has the desired commit history, use the promote-branch workflow
# Follow the steps in the "promote-branch-manually.sh" section above:
#   - Step 1: Create enterprise-patch branch via GitHub Actions (leave branch name empty)
#   - Step 2: Push your local main to the created branch
#   - Step 3: Create PR and get approval
#   - Step 4: Promote to main via GitHub Actions
```

> [!NOTE]
> Any modifications to enterprise patches should be done carefully and reviewed thoroughly, as they affect the delta between community and enterprise codebases.

## History & Audit

View historical sync points and rollback if needed:

```bash
# View historical sync points
git tag -l "community-sync-history-*"

# Example: View a specific sync point from above list
git show community-sync-history-2024-01-15-143022
```

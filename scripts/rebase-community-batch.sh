#!/bin/bash
# Unified community rebase script - consolidated batch processing
# This script handles all community rebase operations in a single unified flow
# The design goals are https://www.notion.so/paradedb/Patch-Based-Enterprise-Workflow-Proposal-23aea4ce9deb80d7a137cbaaeefcfc65?d=26fea4ce9deb8027b999001c944db2f7#255ea4ce9deb805bb208d21eb3390518
set -Eeuo pipefail

# --- Error Handling ---
# Trap errors for detailed logging
trap 'error_handler $? $LINENO' ERR

# shellcheck disable=SC2317,SC2329  # Function invoked indirectly via trap
error_handler() {
  local exit_code="$1"
  local line_number="$2"
  log_error "Error in ${BASH_SOURCE[0]} at line $line_number: Command failed with exit code $exit_code."
  # It's often useful to see the command that failed, which BASH_COMMAND can provide.
  if [[ -n "$BASH_COMMAND" ]]; then
    log_error "Failing command: $BASH_COMMAND"
  fi
}

# --- Configuration ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# TODO: Remove this limit.
MAX_COMMITS=500
# shellcheck source=./rebase-community-helpers.sh
# shellcheck disable=SC1091
source "$SCRIPT_DIR/rebase-community-helpers.sh"

IS_DRY_RUN=false
DEBUG=false
SKIP_VALIDATION=false
while [[ "$#" -gt 0 ]]; do
  case $1 in
    --max-commits)
      shift
      MAX_COMMITS="$1"
      if ! [[ "$MAX_COMMITS" =~ ^[0-9]+$ ]] || [[ "$MAX_COMMITS" -lt 1 ]]; then
        log_error "max-commits must be a positive integer"
        exit 1
      fi
      ;;
    --skip-validation)
      SKIP_VALIDATION=true
      ;;
    --dry-run)
      IS_DRY_RUN=true
      ;;
    --debug)
      DEBUG=true
      export DEBUG
      ;;
    -h|--help)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Unified community rebase script that applies multiple commits with CI validation."
      echo ""
      echo "Options:"
      echo "  --max-commits N       Maximum number of commits to process (default: 5)"
      echo "  --skip-validation     Skip CI validation step"
      echo "  --dry-run             Show what would be done without making changes"
      echo "  --debug               Enable debug logging"
      echo "  -h, --help            Show this help message"
      echo ""
      echo "Examples:"
      echo "  $0                              # Apply up to 5 commits"
      echo "  $0 --max-commits 5              # Apply up to 5 commits"
      echo "  $0 --dry-run --max-commits 3    # Preview up to 3 commits"
      echo "  $0 --skip-validation            # Skip CI validation"
      exit 0
      ;;
    *)
      log_error "Unknown argument: $1"
      echo "Use --help for usage information"
      exit 1
      ;;
  esac
  shift
done

# --- Validation ---
# Validate git repository state before starting
if ! validate_git_state; then
  log_error "Git repository validation failed"
  exit 1
fi

# --- Initial Setup ---
log_info "Setting up community remote and fetching latest changes..."
setup_community_remote
get_pending_commits_count() {
  local common_ancestor
  # Compare current branch (HEAD) with community/main to see remaining commits
  if common_ancestor=$(get_common_ancestor "HEAD" "community/main" 2>/dev/null); then
    count_pending_commits "$common_ancestor" "community/main" 2>/dev/null || echo "0"
  else
    echo "0"
  fi
}

log_info "Creating pre-sync history tag for audit trail..."
# Use UTC for consistent timestamps across different timezones/environments
TAG_DATE=$(date -u +%Y-%m-%d)
TAG_TIME=$(date -u +%H%M%S)
TAG_NAME="community-sync-history-${TAG_DATE}-${TAG_TIME}"

# Get basic sync information first
local_common_ancestor=""
if local_common_ancestor=$(get_common_ancestor "origin/main" "community/main" 2>/dev/null); then
  # Get total pending commits count
  TOTAL_PENDING=$(count_pending_commits "$local_common_ancestor" "community/main" 2>/dev/null || echo "0")

  # Early exit if no commits to process
  if [[ "$TOTAL_PENDING" -eq 0 ]]; then
    log_success "No commits to process. Repository is already up to date with community."
    exit 0
  fi

  # Create and push the tag (only if not dry run)
  if [[ "$IS_DRY_RUN" == "false" ]]; then
    git tag "$TAG_NAME"
    git push origin "$TAG_NAME"
    log_success "Tag pushed to remote: $TAG_NAME"
  else
    log_info "Would create tag: $TAG_NAME (dry run)"
  fi
else
  log_error "Could not determine common ancestor with community. Cannot proceed."
  exit 1
fi

# --- Main Processing Loop ---
log_info "Starting batch sync (max: $MAX_COMMITS commits)..."

if [[ "$IS_DRY_RUN" == "true" ]]; then
  log_info "DRY RUN MODE: No changes will be made"
fi

# ========================================
# CREATE PATCH BRANCH FROM ORIGIN/MAIN
# ========================================
log_debug "Creating patch branch from origin/main (local main remains untouched)..."
git fetch origin

# Get common ancestor between enterprise and community
COMMON_ANCESTOR=$(get_common_ancestor_or_exit "origin/main" "community/main")

# Create patch branch name
PATCH_BRANCH_NAME="enterprise-patch-$(date -u +%Y-%m-%d-%H%M%S)"
log_info "Creating patch branch: $PATCH_BRANCH_NAME"

if [[ "$IS_DRY_RUN" == "false" ]]; then
  # Create patch branch from origin/main (not from local main)
  git checkout -b "$PATCH_BRANCH_NAME" origin/main
  log_success "Created and checked out patch branch: $PATCH_BRANCH_NAME"
else
  log_info "Would create patch branch: $PATCH_BRANCH_NAME from origin/main (dry run)"
fi

CURRENT_HEAD=$(git rev-parse --short HEAD)
COMMUNITY_HEAD=$(git rev-parse --short community/main)

# Get counts and ensure they're clean numeric values
TOTAL_PENDING=$(count_pending_commits "$COMMON_ANCESTOR" "community/main" 2>/dev/null | tail -1 | tr -d '\n\r' || echo "0")
APPLIED_COUNT=$(count_applied_commits "$COMMON_ANCESTOR" "community/main" "HEAD" 2>/dev/null | tail -1 | tr -d '\n\r' || echo "0")

# Ensure we have valid numbers and no garbage creeps in
[[ "$TOTAL_PENDING" =~ ^[0-9]+$ ]] || TOTAL_PENDING=0
[[ "$APPLIED_COUNT" =~ ^[0-9]+$ ]] || APPLIED_COUNT=0

REMAINING_COUNT=$((TOTAL_PENDING - APPLIED_COUNT))

log_debug "📋 Current enterprise HEAD: $CURRENT_HEAD"
log_debug "🎯 Target community HEAD: $COMMUNITY_HEAD"

if [[ "$TOTAL_PENDING" -gt 0 ]]; then
  log_debug "✅ Progress: $APPLIED_COUNT/$TOTAL_PENDING commits applied ($REMAINING_COUNT remaining)"
  log_debug "📊 Sync range: $(git rev-parse --short "$COMMON_ANCESTOR")..$COMMUNITY_HEAD ($TOTAL_PENDING commits total)"
else
  log_info "✅ All commits up to date (no pending commits)"
  exit 0
fi

# ========================================
# COMMIT PROCESSING LOOP
# ========================================
PROCESSED_COUNT=0
START_TIME=$(date +%s)

for ((attempt=0; attempt < MAX_COMMITS; attempt++)); do
  log_info ""
  log_info "=== Processing commit #$((PROCESSED_COUNT + 1)) ==="

  # Get next unapplied commit while tolerating empty results
  NEXT_COMMIT=""
  if ! NEXT_COMMIT=$(get_next_unapplied_commit "$COMMON_ANCESTOR" "community/main" "HEAD" 2>/dev/null); then
    NEXT_COMMIT=""
  fi

  if [[ -z "$NEXT_COMMIT" ]]; then
    log_debug "No unapplied commit found for attempt $((attempt + 1))"
    continue
  fi

  # Validate commit exists
  if ! git cat-file -e "$NEXT_COMMIT" 2>/dev/null; then
    log_error "Commit $NEXT_COMMIT does not exist"
    exit 1
  fi

  # Validate the commit is an ancestor of community/main
  if ! git merge-base --is-ancestor "$NEXT_COMMIT" "community/main"; then
    log_error "Commit $NEXT_COMMIT is not an ancestor of community/main. This should not happen."
    exit 1
  fi

  # Show commit info
  log_debug "Next commit to apply: ${NEXT_COMMIT}"
  show_commit_info "$NEXT_COMMIT"

  # Check for conflicts (dry run mode)
  if [[ "$IS_DRY_RUN" == "true" ]]; then
    log_debug "Checking for potential conflicts..."
    CONFLICT_CHECK=$(git merge-tree "$(git merge-base HEAD "$NEXT_COMMIT")" HEAD "$NEXT_COMMIT" 2>/dev/null)
    if echo "$CONFLICT_CHECK" | grep -q "^<<<<<<<\|^======\|^>>>>>>>"; then
      log_warning "⚠️  Potential conflicts detected for this commit"
      log_info "🔍 Conflict preview:"
      echo "$CONFLICT_CHECK" | grep -A5 -B5 "^<<<<<<<\|^======\|^>>>>>>>" | head -20
    else
      log_debug "✅ No conflicts expected for this commit"
    fi
    PROCESSED_COUNT=$((PROCESSED_COUNT + 1))
    continue
  fi

  # Store HEAD before applying (for progress tracking)
  BEFORE_HEAD=$(git rev-parse HEAD 2>/dev/null || echo "")

  # Apply the commit via rebase
  log_debug "Applying community commit: $NEXT_COMMIT"
  log_debug "Current HEAD before rebase: $BEFORE_HEAD"

  if ! git rebase "$NEXT_COMMIT"; then
    log_error ""
    log_error "🛑 Rebase conflict detected on branch: $PATCH_BRANCH_NAME"
    log_error ""
    log_error "You are currently on the patch branch with conflicts."
    log_error "To resolve:"
    log_error "1. Run this workflow locally by ./scripts/rebase-community-batch.sh. Use --help for options to readme.md in scripts."
    log_error "2. This will leave a conflicted rebase. "
    log_error "3. Resolve conflicts in affected files & complete the rebase."
    log_error "4. Push your fixes to the branch enterprise-patch-<>"
    log_error "5. Use manual promotion workflow in GitHub Actions to land this new branch to main once CI passes."
    log_error ""

    CONFLICT_CONTENT="/tmp/rebase-conflict-details.json"

    # Get commit details
    COMMIT_AUTHOR=$(git log -1 --format="%an" "$NEXT_COMMIT" 2>/dev/null || echo "Unknown")
    COMMIT_MESSAGE=$(git log -1 --format="%s" "$NEXT_COMMIT" 2>/dev/null || echo "Unknown")

    # Truncate commit message to 100 characters
    if [[ ${#COMMIT_MESSAGE} -gt 100 ]]; then
      COMMIT_MESSAGE="${COMMIT_MESSAGE:0:100}..."
    fi

    # Generate JSON
    cat > "$CONFLICT_CONTENT" << EOF
{
  "commit_author": "$COMMIT_AUTHOR",
  "commit_message": "$COMMIT_MESSAGE"
}
EOF

    if [[ "$DEBUG" == "true" ]]; then
      log_debug "Conflicted commit details:"
      show_commit_info "$NEXT_COMMIT"
      log_debug "Conflict details written to: $CONFLICT_CONTENT"
      if [[ -f "$CONFLICT_CONTENT" ]]; then
        head -n 5 "$CONFLICT_CONTENT"
      fi
    fi

    exit 1
  fi

  log_debug "Rebase command seems to have succeeded."

  # Verify progress was made
  log_debug "Verifying progress after rebase..."
  rev_parse_exit_code=0
  AFTER_HEAD=$(git rev-parse HEAD 2>/dev/null) || rev_parse_exit_code=$?
  log_debug "git rev-parse HEAD exited with code: $rev_parse_exit_code"

  if [[ $rev_parse_exit_code -ne 0 ]]; then
    log_error "Could not determine HEAD after rebase, despite rebase command succeeding."
    log_error "This indicates an unexpected repository state. Exiting to prevent further issues."
    exit 1
  fi

  log_debug "HEAD after rebase: $AFTER_HEAD"

  if [[ "$BEFORE_HEAD" != "$AFTER_HEAD" && -n "$AFTER_HEAD" ]]; then
    PROCESSED_COUNT=$((PROCESSED_COUNT + 1))
    log_debug "Progress detected. Getting commit subject for $NEXT_COMMIT"
    commit_subject=$(git log -1 --format="%s" "$NEXT_COMMIT" 2>/dev/null || echo "Unknown")
    log_debug "Commit subject: '$commit_subject'"
    log_success "Successfully applied commit #$PROCESSED_COUNT: $commit_subject"
    log_debug "HEAD moved from $BEFORE_HEAD to $AFTER_HEAD"
  else
    log_info "No progress detected (HEAD didn't change). Sync may be complete."
    log_debug "BEFORE_HEAD=$BEFORE_HEAD, AFTER_HEAD=$AFTER_HEAD"
    break
  fi

  sleep 1
done

# --- CI Validation (only if commits were processed and not dry run and not skipped) ---
if [[ "$IS_DRY_RUN" == "false" && "$PROCESSED_COUNT" -gt 0 && "$SKIP_VALIDATION" == "false" ]]; then
  log_info ""
  log_info "=========================================="
  log_info "CI VALIDATION"
  log_info "=========================================="

  # Push patch branch to trigger CI
  log_info "Pushing patch branch to trigger CI..."
  if git push --set-upstream origin "$PATCH_BRANCH_NAME"; then
    # Poll CI status on the patch branch
    log_info "Waiting for CI validation to complete..."
    log_info "This may take up to 50 minutes. Checking every 5 minutes."

    if poll_branch_ci_status "$PATCH_BRANCH_NAME"; then
      # CI passed - push patch branch to main and cleanup
      log_success "CI validation passed! Promoting patch to main..."

      # Push patch branch to main
      log_info "Pushing patch branch to main..."
      if git push origin "$PATCH_BRANCH_NAME:main" --force-with-lease; then
        # Delete patch branch (both remote and local)
        log_info "Cleaning up patch branch..."
        git push origin --delete "$PATCH_BRANCH_NAME" 2>/dev/null || log_warning "Failed to delete remote patch branch"
        # Switch to a detached HEAD state on origin/main instead of checking out local main
        git checkout origin/main 2>/dev/null || log_warning "Failed to checkout origin/main"
        git branch -D "$PATCH_BRANCH_NAME" 2>/dev/null || log_warning "Failed to delete local patch branch"

        log_success "✅ All commits successfully processed and validated!"
      else
        log_error "Failed to push patch branch to main"
        exit 1
      fi
    else
      # CI failed - preserve patch branch for investigation
      log_error ""
      log_error "❌ CI VALIDATION FAILED"
      log_error "Patch branch '$PATCH_BRANCH_NAME' preserved for investigation."
      log_error ""
      log_error "To investigate:"
      log_error "1. View failed checks: $(git remote get-url origin | sed 's/\.git$//' | sed 's/github\.com:/github\.com\//')/actions"
      log_error "2. Check out the branch: git checkout $PATCH_BRANCH_NAME"
      log_error "3. Fix issues and push your fixes"
      log_error "4. When CI passes, manually merge to main or create a PR"
      log_error ""

      exit 1
    fi
  else
    log_error "Failed to push patch branch to trigger CI"
    git checkout origin/main 2>/dev/null || log_warning "Failed to checkout origin/main"
    git branch -D "$PATCH_BRANCH_NAME" 2>/dev/null
    exit 1
  fi
elif [[ "$IS_DRY_RUN" == "false" && "$PROCESSED_COUNT" -gt 0 && "$SKIP_VALIDATION" == "true" ]]; then
  log_info ""
  log_info "=========================================="
  log_info "CI VALIDATION SKIPPED"
  log_info "=========================================="
  log_info "CI validation was skipped by user request."

  # Push patch branch directly to main and cleanup
  log_info "Pushing patch branch directly to main..."
  if git push origin "$PATCH_BRANCH_NAME:main" --force-with-lease; then
    # Delete patch branch (both remote and local)
    log_info "Cleaning up patch branch..."
    git checkout origin/main 2>/dev/null || log_warning "Failed to checkout origin/main"
    git branch -D "$PATCH_BRANCH_NAME" 2>/dev/null || log_warning "Failed to delete local patch branch"

    log_success "✅ Commits processed and pushed successfully (CI validation skipped)!"
  else
    log_error "Failed to push patch branch to main"
    exit 1
  fi
fi

# --- Summary ---
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Get remaining commits count for summary (without switching branches)
log_debug "Checking for remaining pending commits..."
PENDING_COMMITS=$(get_pending_commits_count)
log_debug "Pending commits remaining: $PENDING_COMMITS"

log_info ""
log_info "=========================================="
if [[ "$IS_DRY_RUN" == "true" ]]; then
  log_info "BATCH SYNC SUMMARY (Dry run)"
else
  log_info "BATCH SYNC SUMMARY"
fi
log_info "=========================================="
log_success "Commits processed: $PROCESSED_COUNT"
log_info "Duration: ${DURATION} seconds"

# Show tag information in summary
if [[ "$IS_DRY_RUN" == "false" && "$PROCESSED_COUNT" -gt 0 && -n "${TAG_NAME:-}" ]]; then
  log_info "Pre-sync history tag: $TAG_NAME"
  log_info "💡 View original state anytime using: git show $TAG_NAME"
fi

if [[ "$PROCESSED_COUNT" -eq 0 ]]; then
  log_info "No commits were processed (may already be up to date)"
elif [[ "$PROCESSED_COUNT" -eq "$MAX_COMMITS" ]]; then
  if [[ "$PENDING_COMMITS" -gt 0 ]]; then
    log_info "Reached maximum commit limit. 🚨 $PENDING_COMMITS commits remaining."
  else
    log_info "Reached maximum commit limit. Run again to continue if more commits remain."
  fi
else
  if [[ "$PENDING_COMMITS" -gt 0 ]]; then
    log_success "Batch processing completed! 🚨 $PENDING_COMMITS commits remaining."
  else
    log_success "Batch processing completed!"
  fi
fi

log_debug "Batch script exiting successfully"
exit 0

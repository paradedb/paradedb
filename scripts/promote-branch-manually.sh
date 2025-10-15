#!/bin/bash
# Promote a CI-validated branch to main
#
# This script promotes a successful CI-validated branch (enterprise-patch-*) to main branch.
#
# Safety Mechanism:
# - Uses --force-with-lease as the primary safety check when pushing to main
# - If --force-with-lease fails, checks whether the failure is due to community-only changes:
#   * Compares merge-bases: git merge-base community/main origin/main vs git merge-base community/main branch
#   * If different: main has newer community commits (safe to override and force push)
#   * If same: main has enterprise changes/concurrent promotion (unsafe - exit with error)
#
# Workflow:
# 1. Validate branch name format and existence
# 2. Poll GitHub Actions CI until all checks pass
# 3. Create backup tag at current main position
# 4. Attempt force-push with --force-with-lease protection
# 5. If lease fails, check if only community sync occurred (allow override if so)
# 6. Clean up promoted branch from remote
#
set -Eeuo pipefail

if [[ "${DEBUG:-false}" == "true" ]]; then
  set -x
fi

# --- Configuration ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./rebase-community-helpers.sh
# shellcheck disable=SC1091
source "$SCRIPT_DIR/rebase-community-helpers.sh"

DRY_RUN=false
DEBUG=true
BRANCH_NAME=""
POLL_TIMEOUT=3000  # Default 50 minutes
POLL_INTERVAL=300  # Default 5 minutes

# --- Argument Parsing ---
while [[ "$#" -gt 0 ]]; do
  case $1 in
    --branch)
      shift
      BRANCH_NAME="$1"
      ;;
    --dry-run)
      DRY_RUN=true
      ;;
    --debug)
      DEBUG=true
      export DEBUG
      ;;
    -h|--help)
      echo "Usage: $0 --branch <branch-name> [OPTIONS]"
      echo ""
      echo "Promote a CI-validated branch to main branch."
      echo ""
      echo "Required:"
      echo "  --branch NAME        Name of the branch to promote (must match enterprise-patch-*)"
      echo ""
      echo "Options:"
      echo "  --dry-run           Show what would be done without making changes"
      echo "  --debug             Enable debug logging"
      echo "  -h, --help          Show this help message"
      echo ""
      echo "Examples:"
      echo "  $0 --branch enterprise-patch-2025-10-07-123456"
      echo "  $0 --branch enterprise-patch-2025-10-07-123456 --dry-run"
      echo ""
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
if [[ -z "$BRANCH_NAME" ]]; then
  log_error "Branch name is required. Use --branch <branch-name>"
  echo "Use --help for usage information"
  exit 1
fi

# Validate branch name format
if ! [[ "$BRANCH_NAME" =~ ^enterprise-patch-[0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]{6}$ ]]; then
  log_error "Invalid branch name format: $BRANCH_NAME"
  log_error "Branch name must match pattern: enterprise-patch-YYYY-MM-DD-HHMMSS"
  log_error "Example: enterprise-patch-2025-10-07-123456"
  exit 1
fi

log_debug "Starting branch promotion process..."
if [[ "$DRY_RUN" == "true" ]]; then
  log_info "DRY RUN MODE: No changes will be made"
fi

# Fetch latest changes from origin
log_info "Fetching latest changes from origin..."
git fetch origin

# Verify branch exists remotely
log_info "Verifying branch '$BRANCH_NAME' exists on remote..."
if ! git ls-remote --exit-code --heads origin "$BRANCH_NAME" >/dev/null 2>&1; then
  log_error "Branch '$BRANCH_NAME' does not exist on remote origin"
  log_error "Please verify the branch name and try again"
  exit 1
fi

log_success "Branch '$BRANCH_NAME' found on remote"

# Fetch the branch if it doesn't exist locally
if ! git show-ref --verify --quiet "refs/heads/$BRANCH_NAME"; then
  log_info "Branch not found locally, fetching from origin..."
  git fetch origin "$BRANCH_NAME:$BRANCH_NAME"
else
  log_debug "Branch exists locally, updating from origin..."
  git fetch origin "$BRANCH_NAME"
fi

# Get branch commit SHA for logging
BRANCH_SHA=$(git rev-parse --short "$BRANCH_NAME")
log_info "Branch '$BRANCH_NAME' points to commit: $BRANCH_SHA"

# --- CI Status Polling ---
log_info ""
log_info "=========================================="
log_info "CI VALIDATION CHECK"
log_info "=========================================="
log_info "Polling CI status for branch: $BRANCH_NAME"
log_info "Timeout: ${POLL_TIMEOUT}s, Interval: ${POLL_INTERVAL}s"

if [[ "$DRY_RUN" == "true" ]]; then
  log_info "Would poll CI status for branch: $BRANCH_NAME (dry run)"
  log_info "Would wait for all CI checks to complete (dry run)"
else
  # Poll CI status using shared helper function
  if poll_branch_ci_status "$BRANCH_NAME" "$POLL_TIMEOUT" "$POLL_INTERVAL"; then
    log_success "CI validation passed!"
  else
    CI_EXIT_CODE=$?
    log_error ""
    log_error "❌ CI VALIDATION FAILED OR TIMED OUT"
    log_error "Branch '$BRANCH_NAME' will NOT be promoted to main"
    log_error ""
    log_error "Next steps:"
    log_error "1. Check CI logs for details"
    log_error "2. Fix any issues in the branch"
    log_error "3. Push fixes to '$BRANCH_NAME'"
    log_error "4. Re-run this script to try again"
    exit $CI_EXIT_CODE
  fi
fi

# --- Create Backup Tag ---
log_info ""
log_info "=========================================="
log_info "CREATING BACKUP TAG"
log_info "=========================================="

TAG_DATE=$(date -u +%Y-%m-%d)
TAG_TIME=$(date -u +%H%M%S)
TAG_NAME="manual-promotion-history-${TAG_DATE}-${TAG_TIME}"

CURRENT_MAIN_SHA=$(git rev-parse --short origin/main)
log_info "Current main branch points to: $CURRENT_MAIN_SHA"
log_info "Creating backup tag: $TAG_NAME"

if [[ "$DRY_RUN" == "true" ]]; then
  log_info "Would create tag: $TAG_NAME at origin/main (dry run)"
  log_info "Would push tag to remote (dry run)"
else
  # Create tag at current origin/main position
  git tag "$TAG_NAME" origin/main
  git push origin "$TAG_NAME"
  log_success "Backup tag created and pushed: $TAG_NAME"
  log_info "💡 Rollback anytime using: git reset --hard $TAG_NAME"
fi

# --- Promote Branch to Main ---
log_info ""
log_info "=========================================="
log_info "PROMOTING BRANCH TO MAIN"
log_info "=========================================="
log_info "Promoting '$BRANCH_NAME' to main branch..."
log_info "This will force main to point to: $BRANCH_SHA"
log_info ""
log_info "Safety: Using --force-with-lease to detect concurrent changes"
log_info "If main was updated by community sync only, promotion will proceed automatically"
log_info ""

if [[ "$DRY_RUN" == "true" ]]; then
  log_info "Would execute: git push --force-with-lease origin $BRANCH_NAME:main (dry run)"
  log_info "Would delete branch from origin: git push origin --delete $BRANCH_NAME (dry run)"
else
  log_info "Force pushing branch to origin/main (with lease protection)..."

  if git push --force-with-lease origin "$BRANCH_NAME:main"; then
    log_success "✅ Successfully promoted '$BRANCH_NAME' to origin/main!"

    # Clean up the promoted branch from origin
    log_info "Deleting promoted branch from origin..."
    if git push origin --delete "$BRANCH_NAME"; then
      log_success "Deleted branch '$BRANCH_NAME' from origin"
    else
      log_warning "Failed to delete branch from origin (non-critical), Please do it yourself"
    fi
  else
    # Force-with-lease failed - check if it's due to community sync only
    log_info ""
    log_info "Force-with-lease failed. Checking if only community changes are present..."

    # Setup and fetch community remote to get latest community/main
    log_debug "Setting up community remote..."
    setup_community_remote

    # Compare merge-bases with community to detect if only community changed
    #
    # We compare where origin/main and the branch each diverged from community/main:
    # - If merge-bases are DIFFERENT: origin/main has newer community commits than branch
    #   → This means a community sync happened on main (safe to override lease)
    # - If merge-bases are SAME: origin/main and branch share the same community base
    #   → This means an enterprise-only change happened on main (concurrent promotion - NOT safe)
    #
    MAIN_COMMUNITY_BASE=$(git merge-base community/main origin/main 2>/dev/null || echo "")
    BRANCH_COMMUNITY_BASE=$(git merge-base community/main "$BRANCH_NAME" 2>/dev/null || echo "")

    if [[ -z "$MAIN_COMMUNITY_BASE" || -z "$BRANCH_COMMUNITY_BASE" ]]; then
      log_error ""
      log_error "❌ UNABLE TO DETERMINE MERGE-BASE WITH COMMUNITY"
      log_error "Could not find common ancestor with community/main"
      log_error "This might indicate the community remote is not properly configured"
      log_error ""
      log_error "Recovery steps:"
      log_error "1. Verify community remote exists: git remote -v"
      log_error "2. Fetch community: git fetch community"
      log_error "3. Check merge-base manually: git merge-base community/main origin/main"
      exit 1
    fi

    if [[ "$MAIN_COMMUNITY_BASE" != "$BRANCH_COMMUNITY_BASE" ]]; then
      # Different community bases = only community sync happened on main
      log_info ""
      log_info "✅ Detected community-only changes on main"
      log_info "Main's community base: $(git rev-parse --short "$MAIN_COMMUNITY_BASE")"
      log_info "Branch's community base: $(git rev-parse --short "$BRANCH_COMMUNITY_BASE")"
      log_info ""
      log_info "Overriding lease protection and forcing push..."
      log_info "This is safe because only community sync occurred (manual approval process in place)"

      if git push --force origin "$BRANCH_NAME:main"; then
        log_success "✅ Successfully promoted '$BRANCH_NAME' to origin/main!"

        # Clean up the promoted branch from origin
        log_info "Deleting promoted branch from origin..."
        if git push origin --delete "$BRANCH_NAME"; then
          log_success "Deleted branch '$BRANCH_NAME' from origin"
        else
          log_warning "Failed to delete branch from origin (non-critical), Please do it yourself"
        fi
      else
        log_error ""
        log_error "❌ FORCE PUSH FAILED (even without lease protection)"
        log_error "This is an unexpected error - please investigate"
        exit 1
      fi
    else
      # Same community base = enterprise changes happened (concurrent promotion)
      log_error ""
      log_error "❌ CONCURRENT ENTERPRISE CHANGES DETECTED"
      log_error "The remote main branch has enterprise-only changes"
      log_error "This indicates a concurrent promotion may have occurred"
      log_error ""
      log_error "Main's community base: $(git rev-parse --short "$MAIN_COMMUNITY_BASE")"
      log_error "Branch's community base: $(git rev-parse --short "$BRANCH_COMMUNITY_BASE")"
      log_error ""
      log_error "Recovery steps:"
      log_error "1. Fetch latest changes: git fetch origin"
      log_error "2. Review origin/main history: git log origin/main"
      log_error "3. Coordinate with your team about the concurrent promotion"
      log_error "4. Rebase your branch if needed and re-run this script"
      exit 1
    fi
  fi
fi

# --- Summary ---
log_info ""
log_info "=========================================="
if [[ "$DRY_RUN" == "true" ]]; then
  log_info "PROMOTION SUMMARY (Dry run)"
else
  log_info "PROMOTION SUMMARY"
fi
log_info "=========================================="
log_success "Branch: $BRANCH_NAME"
log_success "Commit: $BRANCH_SHA"

if [[ "$DRY_RUN" == "false" ]]; then
  log_success "Backup tag: $TAG_NAME"
  log_info ""
  log_info "The validation branch '$BRANCH_NAME' has been deleted from origin"
  log_info "If you need to restore it, you can create it from the backup tag:"
  log_info "  git checkout -b $BRANCH_NAME $TAG_NAME"
  log_info ""
  log_success "✅ Main branch successfully promoted!"
else
  log_info ""
  log_info "This was a dry run. No changes were made."
  log_info "Run without --dry-run to perform the actual promotion."
fi

log_debug "Promotion script completed successfully"
exit 0

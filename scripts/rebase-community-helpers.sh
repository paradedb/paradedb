#!/bin/bash
# Shared utilities for community sync scripts
# This file contains common functions used across sync scripts

set -Eeuo pipefail

# --- Configuration Constants ---
# shellcheck disable=SC2034
ENTERPRISE_BRANCH="main"
COMMUNITY_REPO_URL="https://github.com/paradedb/paradedb.git"

# All timestamps are in UTC

log_info() {
  echo "ℹ️  [$(date -u '+%Y-%m-%d %H:%M:%S')] $1"
}

log_success() {
  echo "✅ [$(date -u '+%Y-%m-%d %H:%M:%S')] $1"
}

log_warning() {
  echo "⚠️  [$(date -u '+%Y-%m-%d %H:%M:%S')] $1"
}

log_error() {
  echo "❌ [$(date -u '+%Y-%m-%d %H:%M:%S')] $1" >&2
}

log_debug() {
  if [[ "${DEBUG:-false}" == "true" ]]; then
    echo "🐛 [$(date -u '+%Y-%m-%d %H:%M:%S')] DEBUG: $1" >&2
  fi
  return 0
}

get_common_ancestor() {
  local enterprise_ref="${1:-origin/main}"
  local community_ref="${2:-community/main}"

  log_debug "Finding common ancestor between $enterprise_ref and $community_ref"

  # Verify both refs exist
  if ! git rev-parse --verify "$enterprise_ref" >/dev/null 2>&1; then
    log_debug "Enterprise ref $enterprise_ref does not exist"
    return 1
  fi

  if ! git rev-parse --verify "$community_ref" >/dev/null 2>&1; then
    log_debug "Community ref $community_ref does not exist"
    return 1
  fi

  local ancestor
  ancestor=$(git merge-base "$enterprise_ref" "$community_ref" 2>/dev/null)
  if [[ $? -eq 0 && -n "$ancestor" ]]; then
    log_debug "Common ancestor found: $ancestor"
    echo "$ancestor"
    return 0
  else
    log_debug "Failed to find common ancestor"
    return 1
  fi
}

# Get and validate common ancestor with proper error handling and logging
get_common_ancestor_or_exit() {
  local enterprise_ref="${1:-origin/main}"
  local community_ref="${2:-community/main}"

  local common_ancestor
  common_ancestor=$(get_common_ancestor "$enterprise_ref" "$community_ref" 2>/dev/null)

  if [[ -z "$common_ancestor" ]]; then
    log_error "Could not find common ancestor between $enterprise_ref and $community_ref"
    log_error "This might indicate:"
    log_error "  1. The repositories have no shared history"
    log_error "  2. One of the references doesn't exist"
    log_error "  3. Network/fetch issues with the community remote"
    log_debug "Try running: git fetch community && git log --oneline --graph $enterprise_ref $community_ref"
    exit 1
  fi

  echo "$common_ancestor"
}

# Get list of community commits that need to be applied (in chronological order)
get_pending_community_commits() {
  local common_ancestor="$1"
  local community_ref="${2:-community/main}"

  log_debug "Getting pending commits from $common_ancestor to $community_ref"
  if [[ -z "$common_ancestor" ]]; then
    log_debug "Common ancestor is empty"
    return 1
  fi

  local commits
  commits=$(git rev-list --reverse "${common_ancestor}..${community_ref}" 2>/dev/null)
  local exit_code=$?

  if [[ $exit_code -eq 0 ]]; then
    if [[ -n "$commits" ]]; then
      log_debug "Found $(echo "$commits" | wc -l | tr -d ' ') pending commits"
      echo "$commits"
      return 0
    else
      log_debug "No pending commits found (ranges are equal)"
      return 1
    fi
  else
    log_debug "Failed to get commit range (exit code: $exit_code)"
    return 1
  fi
}

# Check if a commit has already been applied to the current branch
is_commit_applied() {
  local commit_sha="$1"
  local target_branch="${2:-HEAD}"

  # After rebasing onto a community commit, that commit becomes our HEAD
  # So we need to check if we're already at or past this commit
  local current_head_sha
  current_head_sha=$(git rev-parse "$target_branch" 2>/dev/null)

  # If current HEAD is the exact commit, it's applied
  if [[ "$current_head_sha" == "$commit_sha" ]]; then
    log_debug "Commit $commit_sha is current HEAD - already applied"
    return 0
  fi

  # Check if the commit exists in the target branch's history
  if git merge-base --is-ancestor "$commit_sha" "$target_branch" 2>/dev/null; then
    log_debug "Commit $commit_sha is ancestor of $target_branch - already applied"
    return 0
  fi

  log_debug "Commit $commit_sha is not applied to $target_branch"
  return 1
}

# Get the next unapplied commit from the list
get_next_unapplied_commit() {
  local common_ancestor="$1"
  local community_ref="${2:-community/main}"
  local target_branch="${3:-HEAD}"

  log_debug "Finding next unapplied commit"

  # Get all pending commits
  local pending_commits
  if ! pending_commits=$(get_pending_community_commits "$common_ancestor" "$community_ref"); then
    log_debug "No pending commits found"
    return 1
  fi

  if [[ -z "$pending_commits" ]]; then
    log_debug "No pending commits found"
    return 1
  fi

  # Find the first commit that hasn't been applied
  while IFS= read -r commit; do
    if [[ -n "$commit" ]] && ! is_commit_applied "$commit" "$target_branch"; then
      echo "$commit"
      return 0
    fi
  done <<< "$pending_commits"

  # No unapplied commits found
  return 1
}

# Validate git repository state
validate_git_state() {
  log_debug "Validating git repository state"

  if ! git rev-parse --git-dir > /dev/null 2>&1; then
    log_error "Not in a git repository"
    return 1
  fi

  # Check for uncommitted changes (both tracked and untracked)
  if ! git diff-index --quiet HEAD -- 2>/dev/null || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
    log_error "❌ Uncommitted changes detected in working directory"
    log_error "   Please stash or commit your changes before running this script"
    log_error ""
    log_error "To stash your changes (including untracked files):"
    log_error "   git stash save --include-untracked \"work in progress\""
    log_error ""
    log_error "After the script completes, restore your changes with:"
    log_error "   git stash pop"
    exit 1
  fi
  return 0
}

# Setup community remote if not exists
setup_community_remote() {
  local community_repo_url="${1:-$COMMUNITY_REPO_URL}"

  if ! git remote | grep -q "community"; then
    log_debug "Adding 'community' remote..."
    git remote add community "$community_repo_url"
  fi

  log_debug "Fetching latest from community remote..."
  git fetch community
}

count_pending_commits() {
  local common_ancestor="$1"
  local community_ref="${2:-community/main}"

  local commits
  if commits=$(git rev-list --count "${common_ancestor}..${community_ref}" 2>/dev/null); then
    echo "$commits"
  else
    echo "0" # return 0 commits
  fi
  return 0
}

count_applied_commits() {
  local common_ancestor="$1"
  local community_ref="${2:-community/main}"
  local target_branch="${3:-HEAD}"

  local pending_commits
  if pending_commits=$(get_pending_community_commits "$common_ancestor" "$community_ref"); then
    local applied_count=0
    while IFS= read -r commit; do
      if [[ -n "$commit" ]] && is_commit_applied "$commit" "$target_branch"; then
        ((applied_count++))
      fi
    done <<< "$pending_commits"
    echo "$applied_count"
  else
    echo "0"
  fi
}

get_sync_progress() {
  local common_ancestor="$1"
  local community_ref="${2:-community/main}"
  local target_branch="${3:-HEAD}"

  local total_commits
  total_commits=$(count_pending_commits "$common_ancestor" "$community_ref")

  local applied_commits
  applied_commits=$(count_applied_commits "$common_ancestor" "$community_ref" "$target_branch")

  local remaining_commits=$((total_commits - applied_commits))

  echo "Progress: $applied_commits/$total_commits commits applied ($remaining_commits remaining)"
}

show_commit_info() {
  local commit_sha="$1"
  local commit_subject
  local commit_author
  local commit_date

  commit_subject=$(git log -1 --format="%s" "$commit_sha" 2>/dev/null || echo "Unknown")
  commit_author=$(git log -1 --format="%an" "$commit_sha" 2>/dev/null || echo "Unknown")
  commit_date=$(git log -1 --format="%ad" --date=short "$commit_sha" 2>/dev/null || echo "Unknown")

  echo "Subject: $commit_subject"
  echo "Author: $commit_author, Date: $commit_date"
}



log_exit_status() {
  local exit_code=$?

  # Disable strict error handling in trap to prevent trap logic from affecting exit code
  set +e

  if [[ $exit_code -ne 0 ]]; then
    log_error "Script exited with error code $exit_code"

    # Check if we're in the middle of a rebase
    if git status --porcelain=v1 2>/dev/null | grep -q "^UU\|^AA\|^DD"; then
      log_warning "Repository appears to be in conflict state"
      log_info "Run 'git status' to see conflicted files"
    fi
  else
    log_debug "Script completed successfully with exit code 0"
  fi

  # Re-enable strict mode for any subsequent code (shouldn't be any, but just in case)
  set -e
}

# Get the commit SHA for a branch
get_branch_commit_sha() {
  local branch_name="$1"

  if ! commit_sha=$(git rev-parse "origin/$branch_name" 2>/dev/null); then
    log_error "Branch '$branch_name' not found on remote"
    return 1
  fi

  echo "$commit_sha"
}

# Parse GitHub repository information from remote URL
parse_github_repo_info() {
  local repo_url="$1"

  # Convert various Git URL formats to owner/repo format
  # Examples:
  # - https://github.com/owner/repo.git -> owner/repo
  # - git@github.com:owner/repo.git -> owner/repo
  # - origin -> owner/repo (requires git remote get-url)

  local owner_repo
  if [[ "$repo_url" =~ ^https://github\.com/([^/]+)/([^/]+)\.git$ ]]; then
    owner_repo="${BASH_REMATCH[1]}/${BASH_REMATCH[2]}"
  elif [[ "$repo_url" =~ ^git@github\.com:([^/]+)/([^/]+)\.git$ ]]; then
    owner_repo="${BASH_REMATCH[1]}/${BASH_REMATCH[2]}"
  elif [[ "$repo_url" =~ ^https://github\.com/([^/]+)/([^/]+)$ ]]; then
    owner_repo="${BASH_REMATCH[1]}/${BASH_REMATCH[2]}"
  else
    log_error "Unable to parse GitHub repository URL: $repo_url"
    return 1
  fi

  echo "$owner_repo"
}

# Build jq filter to exclude manual workflow check runs from CI validation
build_ci_filter_excludes() {
  echo "select(.name != \"Rebase Enterprise on Community\" and .name != \"Promote Enterprise Patch Branch to Main\")"
}

# Fetch CI check runs from GitHub API
fetch_ci_check_runs() {
  local owner_repo="$1"
  local commit_sha="$2"


  if ! command -v gh &> /dev/null; then
    log_error "❌ GitHub CLI (gh) is not installed or not in PATH"
    log_error "Please install gh: https://cli.github.com/"
    return 1
  fi

  local auth_output
  auth_output=$(gh auth status 2>&1)
  local auth_exit_code=$?

  if [[ $auth_exit_code -eq 0 ]]; then
    log_debug "✓ [gh auth status] SUCCESS"
  else
    log_debug "[gh auth status] exit code: $auth_exit_code"
    log_debug "Auth output: $auth_output"
  fi

  log_debug "Executing: gh api repos/$owner_repo/commits/$commit_sha/check-runs --paginate"
  local full_response
  local exit_code
  # Use --include to get HTTP headers for status code logging
  full_response=$(gh api "repos/$owner_repo/commits/$commit_sha/check-runs" --paginate --include 2>&1)
  exit_code=$?

  local body
  body=$(echo "$full_response" | sed -n '/^\r$/,$p' | sed '1d')
  local http_status
  http_status=$(echo "$full_response" | grep -E '^HTTP/[0-9.]+\s[0-9]+' | tail -n 1 | awk '{print $2}')

  log_debug "[gh api check-runs] exit code: $exit_code"
  log_debug "[gh api check-runs] HTTP status: $http_status"

  if [[ $exit_code -ne 0 ]]; then
    log_error "❌ FAILED: [gh api check-runs] gh api repos/$owner_repo/commits/$commit_sha/check-runs --paginate"
    log_error "Exit code: $exit_code"
    log_error "HTTP status: $http_status"
    log_error "Error Response: $body"
    return 1
  fi

  log_debug "Response body length: ${#body} characters"

  if [[ -z "$body" ]]; then
    log_warning "API response body is empty, but command succeeded (HTTP status: $http_status)"
  fi

  echo "$body"
}

# Parse check run status from API response
parse_check_run_status() {
  local api_response="$1"

  # Status selectors for readability
  local is_completed='select(.status == "completed")'
  local is_success='select(.status == "completed" and .conclusion == "success")'
  local is_failure='select(.status == "completed" and (.conclusion == "failure" or .conclusion == "cancelled"))'
  local is_pending='select(.status != "completed")'

  # Build jq filter: exclude manual workflows, extract fields, and aggregate status counts
  local jq_filter
  jq_filter="[.check_runs[] | $(build_ci_filter_excludes) | {name: .name, status: .status, conclusion: .conclusion}] | {
    total: length,
    completed: map($is_completed) | length,
    success: map($is_success) | length,
    failure: map($is_failure) | length,
    pending: map($is_pending) | length
  }"

  local status_json
  status_json=$(echo "$api_response" | jq -r "$jq_filter" 2>&1)
  local jq_exit=$?

  if [[ $jq_exit -ne 0 ]]; then
    log_error "❌ Failed to parse API response with jq (exit code: $jq_exit)"
    log_error "jq error: $status_json"
    log_debug "Original API response body (first 500 chars): ${api_response:0:500}"
    return 1
  fi

  log_debug "Parsed status: $status_json"
  echo "$status_json"
}

# Display failed checks for debugging
display_failed_checks() {
  local api_response="$1"

  log_error "Failed CI checks:"
  # Filter: exclude manual workflows, show only completed checks that failed or were cancelled
  echo "$api_response" | jq -r ".check_runs[] | $(build_ci_filter_excludes) | select(.status == \"completed\" and (.conclusion == \"failure\" or .conclusion == \"cancelled\")) | \"  • \(.name): \(.conclusion // \"unknown\")\"" 2>/dev/null || true
}

# Poll GitHub Actions CI status for a branch
poll_branch_ci_status() {
  local branch_name="$1"
  local timeout="${2:-3000}"  # Default 50 minutes (3000 seconds)
  local interval="${3:-300}"   # Default 5 minutes (300 seconds)

  log_debug "Polling CI status for branch: $branch_name"
  log_debug "Timeout: ${timeout}s, Interval: ${interval}s"

  # Get the commit SHA for the branch
  local commit_sha
  if ! commit_sha=$(get_branch_commit_sha "$branch_name"); then
    return 1
  fi

  log_debug "Polling CI status for commit: $commit_sha"

  # Get repository info for GitHub API calls
  local repo_info
  if ! repo_info=$(git remote get-url origin 2>/dev/null); then
    log_error "Failed to get origin URL"
    return 1
  fi

  # Parse GitHub repository information
  local owner_repo
  if ! owner_repo=$(parse_github_repo_info "$repo_info"); then
    return 1
  fi

  log_debug "Repository: $owner_repo"

  local elapsed=0

  while [[ $elapsed -lt $timeout ]]; do
    log_debug "Checking CI status... (${elapsed}s elapsed)"

    # Fetch CI check runs from GitHub API
    local api_response
    if ! api_response=$(fetch_ci_check_runs "$owner_repo" "$commit_sha"); then
      return 1
    fi

    # Parse and analyze check run status
    local status_json
    status_json=$(parse_check_run_status "$api_response")

    # Extract status counts using jq
    local total_checks completed_checks success_checks failure_checks pending_checks
    total_checks=$(echo "$status_json" | jq -r '.total')
    completed_checks=$(echo "$status_json" | jq -r '.completed')
    success_checks=$(echo "$status_json" | jq -r '.success')
    failure_checks=$(echo "$status_json" | jq -r '.failure')
    pending_checks=$(echo "$status_json" | jq -r '.pending')

    log_debug "CI Status: $completed_checks/$total_checks completed, $success_checks success, $failure_checks failure, $pending_checks pending"

    # Check if all checks are completed
    if [[ "$completed_checks" -eq "$total_checks" && "$total_checks" -gt 0 ]]; then
      if [[ "$failure_checks" -gt 0 ]]; then
        log_error "CI validation failed: $failure_checks out of $total_checks checks failed"
        display_failed_checks "$api_response"
        return 1
      else
        log_success "CI validation passed: All $total_checks checks completed successfully"
        return 0
      fi
    fi

    # Wait for next check
    sleep "$interval"
    elapsed=$((elapsed + interval))

    if [[ $elapsed -ge $timeout ]]; then
      break
    fi
  done

  log_error "CI validation timed out after ${timeout}s"
  log_info "Branch '$branch_name' preserved for investigation"
  return 2
}

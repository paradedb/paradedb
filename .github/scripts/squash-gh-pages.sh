#!/bin/bash
#
# Squashes the commit history of the `gh-pages` branch for a specified remote
# into a single orphan root commit containing the exact current file tree.
#
# By default, this script runs safely and prints the resulting commit SHA and
# push command without pushing to any remote. Pass --push to apply the change.
#
# Usage:
#   ./.github/scripts/squash-gh-pages.sh [options] [remote]
#
# Arguments:
#   remote: Git remote name (default: origin)
#
# Options:
#   --push:     Push the squashed root commit to the remote gh-pages branch (force push)
#   --no-fetch: Use existing local tracking ref without fetching from remote
#   -h, --help: Show this help message
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

REMOTE="origin"
DO_PUSH=false
DO_FETCH=true

show_help() {
  sed -n '2,/^$/p' "$0" | tr -d '#'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --push)
      DO_PUSH=true
      shift
      ;;
    --no-fetch)
      DO_FETCH=false
      shift
      ;;
    -h|--help)
      show_help
      exit 0
      ;;
    -*)
      echo "Error: Unknown option '$1'" >&2
      show_help
      exit 1
      ;;
    *)
      REMOTE="$1"
      shift
      ;;
  esac
done

cd "$REPO_ROOT"

if ! git remote get-url "$REMOTE" &>/dev/null; then
  echo "Error: Remote '$REMOTE' does not exist." >&2
  exit 1
fi

if [[ "$DO_FETCH" == true ]]; then
  echo "Fetching tip of gh-pages from $REMOTE (depth 1)..."
  git fetch --depth 1 "$REMOTE" gh-pages:refs/remotes/"$REMOTE"/gh-pages --update-head-ok --quiet
fi

REMOTE_REF="refs/remotes/$REMOTE/gh-pages"
if ! git rev-parse --verify "$REMOTE_REF" &>/dev/null; then
  echo "Error: Branch 'gh-pages' not found on remote '$REMOTE'." >&2
  exit 1
fi

OLD_COMMIT=$(git rev-parse "$REMOTE_REF")
TREE_SHA=$(git rev-parse "${REMOTE_REF}^{tree}")
TIMESTAMP=$(date -u '+%Y-%m-%d %H:%M:%S UTC')

echo "=================================================="
echo " Remote:                  $REMOTE"
echo " Current gh-pages head:   $OLD_COMMIT"
echo " Target Tree SHA:         $TREE_SHA"
echo " Action:                  $([[ "$DO_PUSH" == true ]] && echo "PUSH" || echo "DRY-RUN")"
echo "=================================================="

# Create an orphan root commit (zero parent commits) pointing directly to the tree
COMMIT_MSG="Squash gh-pages history (as of $TIMESTAMP)"
NEW_COMMIT=$(git commit-tree "$TREE_SHA" -m "$COMMIT_MSG")

echo "Created squashed commit: $NEW_COMMIT"
echo ""

if [[ "$DO_PUSH" == true ]]; then
  echo "Force-pushing squashed commit to $REMOTE/gh-pages..."
  git push "$REMOTE" "$NEW_COMMIT:refs/heads/gh-pages" --force
  echo "Successfully squashed and updated gh-pages on $REMOTE."
else
  echo "No changes were pushed to $REMOTE."
  echo "To apply and push this change to $REMOTE, run:"
  echo ""
  echo "  ./.github/scripts/squash-gh-pages.sh --push $REMOTE"
  echo ""
  echo "Or push manually via git:"
  echo "  git push $REMOTE $NEW_COMMIT:refs/heads/gh-pages --force"
fi

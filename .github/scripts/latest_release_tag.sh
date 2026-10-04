#!/bin/bash
# Prints the newest release tag that is not newer than this branch's version.
# A release branch must not upgrade from a later release line, and it cannot build one.
set -euo pipefail

BRANCH_VERSION=$(git show HEAD:Cargo.toml | sed -nE 's/^version = "([^"]+)"$/\1/p' | head -n 1)
if [ -z "$BRANCH_VERSION" ]; then
  echo "Could not read the workspace version from Cargo.toml" >&2
  exit 1
fi

git tag -l 'v*' | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | while read -r tag; do
  newest=$(printf '%s\nv%s\n' "$tag" "$BRANCH_VERSION" | sort -V | tail -n 1)
  if [ "$newest" = "v$BRANCH_VERSION" ]; then
    echo "$tag"
  fi
done | sort -V | tail -n 1

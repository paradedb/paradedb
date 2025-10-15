#!/bin/bash

set -Eeuo pipefail

if ! git remote | grep -q upstream; then
  # we don't have the upstream repo, so add it
  git remote add upstream https://github.com/paradedb/paradedb
fi

git fetch --all || exit "$?"
git checkout community-main || exit "$?"
git pull || exit "$?"
git rebase upstream/main || exit "$?"
git push || exit "$?"
git checkout main || exit "$?"
echo "paradedb-enterprise:community-main sync complete"

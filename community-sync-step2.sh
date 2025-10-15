#!/bin/bash

set -Eeuo pipefail

TS=$(date "+%Y-%m-%d_%H-%M-%S")
BRANCH=community-sync/${TS}
git checkout main || exit "$?"
git pull || exit "$?"
git checkout -b "${BRANCH}" || exit "$?"
if ! git merge community-main
then
  echo "Conflicts detected. Please resolve."
  exit 1
fi

git push -u origin "${BRANCH}" || exit "$?"
echo "Community merge complete. Please make a PR from this new branch to main."
echo
echo "And ensure it's merged with a **merge commit**."

#!/bin/bash

# Fetch the target branch
git fetch origin $VERCEL_GIT_COMMIT_REF

# Check for changes in the ./dashboard directory compared to the target branch
if git diff --quiet origin/$VERCEL_GIT_COMMIT_REF...HEAD -- ./dashboard; then
  # No changes in ./dashboard, exit with 1 to stop the build
  exit 1
else
  # Changes detected, proceed with the build
  exit 0
fi

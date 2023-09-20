#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Retrieve relative subfolder path
DIR="$(dirname "$0")"

cd "$DIR"/../../..

fd --exec .github/workflows/helpers/fix_trailing_newline.sh

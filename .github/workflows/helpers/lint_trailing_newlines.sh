#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Retrieve relative subfolder path
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd "$DIR"/../../..

fd --exec .github/workflows/helpers/fix_trailing_newline.sh

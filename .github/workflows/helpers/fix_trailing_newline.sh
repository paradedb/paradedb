#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

if [[ -f "$1" ]]; then
  echo -n "$1"
  if (diff /dev/null "$1" || true) | tail -1 | grep -q '^\\ No newline'; then
    echo >> "$1"
    echo "...fixed"
  else
    echo ""
  fi
fi

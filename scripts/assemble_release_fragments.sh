#!/bin/bash
# scripts/assemble_release_fragments.sh
#
# Unified wrapper for assembling release fragments (SQL migration scripts, changelogs, and doc versions).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

exec python3 "${REPO_ROOT}/.github/scripts/assemble_release_fragments.py" --repo-root "${REPO_ROOT}" "$@"

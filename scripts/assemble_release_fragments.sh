#!/bin/bash
# scripts/assemble_release_fragments.sh
#
# Unified wrapper for assembling release fragments (SQL migration scripts, changelogs, and doc versions).
#
# Usage:
#   ./scripts/assemble_release_fragments.sh sql <target_version> [--prev-version <version>] [--preserve-fragments]
#   ./scripts/assemble_release_fragments.sh changelog <target_version> [--preserve-fragments] [--register-only] [--is-latest/--no-is-latest]
#   ./scripts/assemble_release_fragments.sh all <target_version> [--preserve-fragments]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

exec python3 "${REPO_ROOT}/.github/scripts/assemble_release_fragments.py" --repo-root "${REPO_ROOT}" "$@"

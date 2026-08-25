#!/usr/bin/env bash
# scripts/assemble_changelog.sh
#
# Assembles unreleased changelog fragments into docs/changelog/<version>.mdx
# and updates docs/docs.json.
#
# Usage:
#   ./scripts/assemble_changelog.sh <target_version> [--dry-run]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

exec python3 "${REPO_ROOT}/.github/scripts/assemble_changelog.py" --repo-root "${REPO_ROOT}" "$@"

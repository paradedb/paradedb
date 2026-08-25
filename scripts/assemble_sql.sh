#!/usr/bin/env bash
# scripts/assemble_sql.sh
#
# Assembles unreleased SQL migration fragments into a versioned upgrade script
# (pg_search/sql/pg_search--<prev>--<target>.sql).
#
# Usage:
#   ./scripts/assemble_sql.sh <target_version> [--prev-version <version>] [--transient]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

exec python3 "${REPO_ROOT}/.github/scripts/assemble_sql.py" --repo-root "${REPO_ROOT}" "$@"

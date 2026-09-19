#!/usr/bin/env bash
# scripts/dev-install.sh
#
# Development wrapper around `cargo pgrx install`.
#
# 1. Runs `cargo pgrx install "$@"` to build the extension, place the shared library
#    in $PKGLIBDIR, and install the base schema and released upgrade scripts into $SHAREDIR/extension.
# 2. Automatically assembles any unreleased SQL fragments (pg_search/sql/unreleased/*.sql)
#    into an ephemeral upgrade script directly in $SHAREDIR/extension, and points the control
#    file's default_version to the target version.
#
# This allows `ALTER EXTENSION pg_search UPDATE;` to work immediately on existing dev databases
# without modifying the repository working copy or dropping existing tables and indexes.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# 1. Run cargo pgrx install
cargo pgrx install "$@"

# 2. Check if there are any unreleased migration fragments
UNRELEASED_DIR="${REPO_ROOT}/pg_search/sql/unreleased"
FRAGMENTS=$(find "${UNRELEASED_DIR}" -name '*.sql' ! -name '.gitkeep' 2>/dev/null || true)
if [ -z "${FRAGMENTS}" ]; then
  exit 0
fi

# 3. Locate pg_config from arguments, environment, or PATH
PG_CONFIG=""
ARGS=("$@")
for ((i = 0; i < ${#ARGS[@]}; i++)); do
  case "${ARGS[i]}" in
    -c | --pg-config)
      if ((i + 1 < ${#ARGS[@]})); then
        PG_CONFIG="${ARGS[i + 1]}"
      fi
      ;;
    --pg-config=*)
      PG_CONFIG="${ARGS[i]#*=}"
      ;;
  esac
done

if [ -z "${PG_CONFIG}" ]; then
  PG_CONFIG="${PG_CONFIG:-$(which pg_config 2>/dev/null || true)}"
fi

if [ -z "${PG_CONFIG}" ] || [ ! -x "${PG_CONFIG}" ]; then
  echo "⚠️  dev-install: Could not find pg_config executable; skipping ephemeral upgrade script installation."
  exit 0
fi

SHAREDIR=$("${PG_CONFIG}" --sharedir)
EXTDIR="${SHAREDIR}/extension"

if [ ! -d "${EXTDIR}" ]; then
  echo "⚠️  dev-install: Extension directory ${EXTDIR} does not exist; skipping ephemeral upgrade script installation."
  exit 0
fi

# 4. Resolve the latest released upgrade target in pg_search/sql and compute the next target version
python3 - <<EOF
import sys
from pathlib import Path

sys.path.insert(0, "${REPO_ROOT}/.github/scripts")
import release

repo_root = Path("${REPO_ROOT}")
ext_dir = Path("${EXTDIR}")
sql_dir = repo_root / "pg_search" / "sql"

targets = release.get_existing_sql_targets(sql_dir)
targets.sort(key=release.parse_semver)
if not targets:
    sys.exit(0)

prev_ver = targets[-1]
t = release.parse_semver(prev_ver)
target_ver = f"{t[0]}.{t[1] + 1}.0"

print(f"dev-install: Assembling unreleased fragments for {prev_ver} -> {target_ver} into {ext_dir}...")
release.assemble_sql_files(
    repo_root=repo_root,
    target_version=target_ver,
    prev_version=prev_ver,
    preserve_fragments=True,
    output_dir=ext_dir,
    update_control_default=True,
)
EOF

echo "✅ dev-install complete: extension and upgrade path installed into ${EXTDIR}."

#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
usage: install_lindera_dictionaries.sh --pg-config PATH [--package-root PATH] [--sudo]

Build Lindera dictionaries as pg_search mmap component files and install them
under $sharedir/extension/pg_search/lindera.

With --package-root, install into the pgrx package tree instead of the live
Postgres sharedir.
USAGE
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pg_config="${PG_CONFIG:-}"
package_root=""
use_sudo=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pg-config)
      pg_config="${2:-}"
      shift 2
      ;;
    --package-root)
      package_root="${2:-}"
      shift 2
      ;;
    --sudo)
      use_sudo=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$pg_config" ]]; then
  pg_config="$(command -v pg_config)"
fi

if [[ ! -x "$pg_config" ]]; then
  echo "pg_config is not executable: $pg_config" >&2
  exit 1
fi

sharedir="$("$pg_config" --sharedir)"
if [[ -n "$package_root" ]]; then
  sharedir_rel="${sharedir#/}"
  destination="$package_root/$sharedir_rel/extension/pg_search/lindera"
else
  destination="$sharedir/extension/pg_search/lindera"
fi

if [[ -z "${LINDERA_CACHE:-}" ]]; then
  if [[ "$use_sudo" -eq 1 ]]; then
    export LINDERA_CACHE="/tmp/paradedb-lindera-dict-cache"
  else
    export LINDERA_CACHE="$repo_root/target/lindera-dict-cache"
  fi
fi

cargo build --release --manifest-path "$repo_root/Cargo.toml" --package lindera-dict-builder

builder="$repo_root/target/release/lindera-dict-builder"
if [[ "$use_sudo" -eq 1 ]]; then
  sudo env LINDERA_CACHE="$LINDERA_CACHE" "$builder" "$destination"
else
  "$builder" "$destination"
fi

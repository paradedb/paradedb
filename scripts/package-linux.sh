#!/usr/bin/env bash
# Build native pgrx packages and retain the release asset filenames.
set -euo pipefail

: "${PG_CONFIG:?}" "${PACKAGE_FORMAT:?}" "${PACKAGE_PATH:?}"
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
pg_major=$("$PG_CONFIG" --version | awk '{split($2, v, "."); print v[1]}')
staged="$root/target/release/pg_search-pg${pg_major}"

# Refuse unassembled fragments, including in tagged release builds.
cargo pgrx package -p pg_search --pg-config "$PG_CONFIG" --no-distrib-packages

if [ "$PACKAGE_FORMAT" = rpm ]; then
  # pgrx's native RPM builder does not scan ELF dependencies as rpmbuild does.
  # Keep those requirements so incompatible system libraries cannot satisfy an install.
  backup=$(mktemp)
  cp pg_search/Cargo.toml "$backup"
  trap 'cp "$backup" pg_search/Cargo.toml; rm -f "$backup"' EXIT
  rpmdeps=/usr/lib/rpm/rpmdeps
  "$rpmdeps" --requires "$staged/usr/pgsql-${pg_major}/lib/pg_search.so" >"$staged/rpm-requires.txt"
  python3 - "$staged/rpm-requires.txt" <<'PY'
import json
import pathlib
import re
import sys

manifest = pathlib.Path("pg_search/Cargo.toml")
requirements = pathlib.Path(sys.argv[1]).read_text().splitlines()
text = manifest.read_text()
start = text.index("[package.metadata.rpm]")
end = text.index("[package.metadata.deb]", start)
section = text[start:end]
section = re.sub(
    r"(?m)^(depends = \[.*)\]$",
    lambda match: match[1] + ", " + ", ".join(json.dumps(dep) for dep in sorted(set(requirements))) + "]",
    section,
)
manifest.write_text(text[:start] + section + text[end:])
PY
fi

cargo pgrx package -p pg_search --pg-config "$PG_CONFIG"
shopt -s nullglob
packages=("$staged"/*."$PACKAGE_FORMAT")
if [ "${#packages[@]}" -ne 1 ]; then
  echo "Expected one native $PACKAGE_FORMAT package under $staged" >&2
  exit 1
fi
cp "${packages[0]}" "$PACKAGE_PATH"

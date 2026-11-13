#!/usr/bin/env bash
# Build native pgrx packages and retain the release asset filenames.
set -euo pipefail

: "${PG_CONFIG:?}" "${PACKAGE_FORMAT:?}" "${PACKAGE_PATH:?}"
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
read -r pg_major pg_minor < <("$PG_CONFIG" --version | awk '{split($2, v, "."); print v[1], v[2] + 0}')
# CVE-2026-6460 moved DatumGetHeapTupleHeader into a server C symbol starting in these
# point releases; pgrx 0.20.0 links against it when built with those headers.
case "$pg_major" in
  15) pg_min_minor=20 ;;
  16) pg_min_minor=16 ;;
  17) pg_min_minor=12 ;;
  18) pg_min_minor=7 ;;
  *)
    echo "Unsupported PostgreSQL major version: $pg_major" >&2
    exit 1
    ;;
esac
pg_min_version="${pg_major}.${pg_min_minor}"
if ((pg_minor < pg_min_minor)); then
  echo "PostgreSQL ${pg_major}.${pg_minor} is older than required floor ${pg_min_version}" >&2
  exit 1
fi
staged="$root/target/release/pg_search-pg${pg_major}"

package_flags=()
# Refuse unassembled fragments on tagged release builds, while still allowing
# manual branch builds when sql/unreleased/ is non-empty.
if [[ "${GITHUB_REF:-}" != refs/tags/v* ]]; then
  package_flags+=(--no-assemble-unreleased)
fi

cargo_version=$(awk -F'"' '/^version = "/ { print $2; exit }' Cargo.toml)
pkg_version_override=""
if [[ -n "${PACKAGE_VERSION:-}" && "$PACKAGE_VERSION" != "$cargo_version" ]]; then
  pkg_version_override=${PACKAGE_VERSION//-/\~}
fi

case "$PACKAGE_FORMAT" in
  deb)
    keep_header="[package.metadata.deb]"
    drop_header="[package.metadata.rpm]"
    pg_dep_match="\"postgresql-\${PG_MAJOR_VERSION}\""
    pg_dep_replacement="\"postgresql-\${PG_MAJOR_VERSION} (>= ${pg_min_version})\""
    ;;
  rpm)
    keep_header="[package.metadata.rpm]"
    drop_header="[package.metadata.deb]"
    # arx-pack passes depends entries to rpm::Dependency::any; parenthesizing as an
    # RPM 4.12+ rich dependency preserves the >= version constraint in rpm and dnf.
    pg_dep_match="\"postgresql\${PG_MAJOR_VERSION}-server\""
    pg_dep_replacement="\"(postgresql\${PG_MAJOR_VERSION}-server >= ${pg_min_version})\""
    ;;
  *)
    echo "Unsupported PACKAGE_FORMAT: $PACKAGE_FORMAT" >&2
    exit 1
    ;;
esac

extra_deps=""
if [ "$PACKAGE_FORMAT" = rpm ]; then
  # pgrx's native RPM builder does not scan ELF dependencies as rpmbuild does.
  # Stage the shared library first so rpmdeps can extract its SONAME requirements.
  cargo pgrx package -p pg_search --pg-config "$PG_CONFIG" --no-distrib-packages "${package_flags[@]}"
  /usr/lib/rpm/rpmdeps --requires "$staged/usr/pgsql-${pg_major}/lib/pg_search.so" >"$staged/rpm-requires.txt"
  extra_deps=$(jq -Rs 'split("\n") | map(select(length > 0)) | unique | map(@json) | join(", ")' "$staged/rpm-requires.txt")
fi

backup=$(mktemp)
cp pg_search/Cargo.toml "$backup"
trap 'cp "$backup" pg_search/Cargo.toml; rm -f "$backup"' EXIT

awk \
  -v keep_header="$keep_header" \
  -v drop_header="$drop_header" \
  -v pkg_ver="$pkg_version_override" \
  -v pg_dep_match="$pg_dep_match" \
  -v pg_dep_replacement="$pg_dep_replacement" \
  -v extra_deps="$extra_deps" '
  BEGIN { skip = 0; in_target = 0 }
  /^\[/ {
    skip = ($0 == drop_header)
    in_target = ($0 == keep_header)
  }
  skip { next }
  in_target && /^depends = \[.*\]$/ {
    idx = index($0, pg_dep_match)
    if (idx == 0) {
      print "Expected " pg_dep_match " in depends line: " $0 > "/dev/stderr"
      exit 1
    }
    $0 = substr($0, 1, idx - 1) pg_dep_replacement substr($0, idx + length(pg_dep_match))
    if (extra_deps != "") {
      $0 = substr($0, 1, length($0) - 1) ", " extra_deps "]"
    }
    print
    next
  }
  {
    print
    if (in_target && $0 == keep_header && pkg_ver != "") {
      printf "version = \"%s\"\n", pkg_ver
    }
  }
' "$backup" >pg_search/Cargo.toml

rm -f "$staged"/*."$PACKAGE_FORMAT"
cargo pgrx package -p pg_search --pg-config "$PG_CONFIG" "${package_flags[@]}"
shopt -s nullglob
packages=("$staged"/*."$PACKAGE_FORMAT")
if [ "${#packages[@]}" -ne 1 ]; then
  echo "Expected one native $PACKAGE_FORMAT package under $staged, found ${#packages[@]}" >&2
  exit 1
fi
cp "${packages[0]}" "$PACKAGE_PATH"

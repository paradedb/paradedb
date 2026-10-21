#!/usr/bin/env bash
# Read the beta target from the signed release tag; older tags default to PG 18.
set -euo pipefail
release_tag="$1"
if [[ "$release_tag" != *-rc.* ]]; then
  echo 18
  exit 0
fi
git fetch --no-tags origin "refs/tags/$release_tag:refs/tags/$release_tag" >&2
pg_version=$(git for-each-ref --format='%(contents)' "refs/tags/$release_tag" | sed -n 's/^Beta-Postgres-Version: //p')
pg_version=${pg_version:-18}
case "$pg_version" in
  15|16|17|18) echo "$pg_version" ;;
  *) echo "Invalid beta PostgreSQL major in $release_tag: $pg_version" >&2; exit 1 ;;
esac

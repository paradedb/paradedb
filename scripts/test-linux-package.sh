#!/usr/bin/env bash
# Exercise a release package through a fresh install and an upgrade from a published package.
set -euo pipefail
: "${PG_CONFIG:?}" "${PACKAGE_FORMAT:?}" "${PACKAGE_PATH:?}" "${OS_VERSION:?}" "${PACKAGE_ARCH:?}"
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
package=$(realpath "$PACKAGE_PATH")
bin=$("$PG_CONFIG" --bindir)
pg_major=$("$PG_CONFIG" --version | awk '{split($2, v, "."); print v[1]}')
version=$(sed -nE 's/^version = "([^"]+)"$/\1/p' Cargo.toml | head -1)
repository=${GITHUB_REPOSITORY:-paradedb/paradedb}
baseline=$(gh release list --repo "$repository" --exclude-drafts --exclude-pre-releases \
  --limit 30 --json tagName | jq -r --arg current "v$version" '[.[] | select(.tagName != $current)][0].tagName')
[ "$baseline" != null ]
work=$(mktemp -d /tmp/pg-search-package.XXXXXX)
chmod 755 "$work"
mkdir "$work/data" "$work/socket"
sudo chown -R postgres "$work/data" "$work/socket"
pg() { sudo -u postgres "$@"; }
cleanup() {
  pg "$bin/pg_ctl" -D "$work/data" -m immediate stop >/dev/null 2>&1 || true
  sudo rm -rf "$work"
}
trap cleanup EXIT

if [ "$PACKAGE_FORMAT" = deb ]; then
  asset="postgresql-${pg_major}-pg-search_${baseline#v}-1PARADEDB-${OS_VERSION}_${PACKAGE_ARCH}.deb"
  install_package() { sudo apt-get install -y --allow-downgrades "$1"; }
  dpkg-deb --info "$package"
  dpkg-deb --contents "$package" >"$work/contents"
  grep -F "/usr/share/doc/postgresql-${pg_major}-pg-search/copyright" "$work/contents"
  # A release candidate must sort before its final release.
  dpkg --compare-versions "${version%%-*}~rc.1" lt "${version%%-*}"
else
  asset="pg_search_${pg_major}-${baseline#v}-1PARADEDB.${OS_VERSION}.${PACKAGE_ARCH}.rpm"
  install_package() { sudo dnf install -y "$1"; }
  rpm -qpi "$package"
  rpm -qp --requires "$package" | grep -F "postgresql${pg_major}-server"
fi

gh release download "$baseline" --repo "$repository" --pattern "$asset" --dir "$work"
install_package "$work/$asset"
pg "$bin/initdb" -D "$work/data" -A trust --no-locale
start() {
  pg "$bin/pg_ctl" -D "$work/data" -l "$work/data/server.log" \
    -o "-c listen_addresses='' -c unix_socket_directories='$work/socket' -c shared_preload_libraries=pg_search" -w start
}
export PGHOST="$work/socket" PGUSER=postgres PGDATABASE=postgres
start
"$bin/psql" -X -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION pg_search CASCADE;
CREATE TABLE package_smoke (id integer PRIMARY KEY, body text);
INSERT INTO package_smoke VALUES (1, 'hello world');
CREATE INDEX package_smoke_idx ON package_smoke USING bm25 (id, body) WITH (key_field = 'id');
SQL
[ "$("$bin/psql" -XAt -c "SELECT count(*) FROM package_smoke WHERE body @@@ 'hello'")" = 1 ]
pg "$bin/pg_ctl" -D "$work/data" -m fast -w stop
install_package "$package"
start
"$bin/psql" -X -v ON_ERROR_STOP=1 -c 'ALTER EXTENSION pg_search UPDATE;'
[ "$("$bin/psql" -XAt -c "SELECT extversion FROM pg_extension WHERE extname = 'pg_search'")" = "$version" ]
[ "$("$bin/psql" -XAt -c "SELECT count(*) FROM package_smoke WHERE body @@@ 'hello'")" = 1 ]
"$bin/createdb" package_fresh
PGDATABASE=package_fresh "$bin/psql" -X -v ON_ERROR_STOP=1 -c 'CREATE EXTENSION pg_search CASCADE;'
[ "$(PGDATABASE=package_fresh "$bin/psql" -XAt -c "SELECT extversion FROM pg_extension WHERE extname = 'pg_search'")" = "$version" ]

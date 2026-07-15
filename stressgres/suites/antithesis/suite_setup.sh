#!/bin/bash
#
# Shared setup, sourced by every first_ command in this template (and by the driver, for
# the constants below). Antithesis runs exactly one first_ command per timeline
# (https://antithesis.com/docs/test_templates/test_composer_reference/), so each first_
# points its suite at the cluster, builds that suite's schema fault-free, and publishes the
# suite's workload at WORKLOAD_LINK. The one singleton_driver then runs whatever was
# published. One suite per timeline means no two suites ever share the paradedb-rw database
# (or its disk) at once.

set -Eeuo pipefail

SUITE_DIR=/home/app/stressgres/suites
STRESSGRES=/home/app/target/release/stressgres

# The paired singleton_driver runs this symlink; each first_ repoints it at its own suite.
WORKLOAD_LINK=/tmp/stressgres-workload.toml

# Point a single-server suite at paradedb-rw. The connection-string query params are
# fail-fast timeouts so a dropped socket lands inside the reconnect-grace window.
rewrite_single() {
  sed -i 's|\[server\.style\.Automatic\]|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' "$1"
}

# Point a logical-replication suite's publisher at the vanilla Postgres pod (an upstream
# primary we do not control) and its subscriber at paradedb-rw (the CNPG primary, has
# pg_search).
rewrite_pub_sub() {
  sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Publisher"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@logical-replication-publisher:5432/postgres?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' "$1"
  sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Subscriber"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' "$1"
}

# vanilla-postgres.toml hardcodes a localhost connection string rather than
# server.style.Automatic, so it needs its own rewrite.
rewrite_vanilla() {
  sed -i 's|postgresql://postgres:postgres@localhost:5432/postgres|postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15|g' "$1"
}

# Point <toml> at its cluster(s) using <topology>, build its schema fault-free, and publish
# it for the paired singleton_driver.
setup() {
  local toml="$1" topology="$2"
  local path="${SUITE_DIR}/${toml}"

  echo ""
  echo "Pointing ${toml} at its cluster(s)..."
  case "${topology}" in
    single)  rewrite_single  "${path}" ;;
    pub_sub) rewrite_pub_sub "${path}" ;;
    vanilla) rewrite_vanilla "${path}" ;;
    *) echo "unknown topology: ${topology}" >&2; exit 1 ;;
  esac

  echo ""
  echo "Waiting 60s for the ParadeDB cluster to initialize..."
  sleep 60

  echo ""
  echo "Building schema for ${toml}..."
  "${STRESSGRES}" headless "${path}" --setup-only --reconnect-grace 200000

  # Publish the workload. Exactly one first_ runs per timeline, so the driver always runs
  # the suite we just built.
  ln -sf "${path}" "${WORKLOAD_LINK}"

  echo ""
  echo "Schema build complete!"
}

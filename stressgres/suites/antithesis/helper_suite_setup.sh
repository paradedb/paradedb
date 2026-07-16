#!/bin/bash
#
# Shared setup for this template's first_ commands (and constants for the driver). Antithesis
# runs exactly one first_ per timeline, so each first_ points its suite at the cluster, builds
# the schema fault-free, and publishes the workload at WORKLOAD_LINK for the singleton_driver.

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

# Point a suite's publisher at the vanilla Postgres pod (an upstream primary we do not
# control).
rewrite_publisher() {
  sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Publisher"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@logical-replication-publisher:5432/postgres?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' "$1"
}

# Point a suite's subscriber at paradedb-rw (the CNPG primary, has pg_search).
rewrite_subscriber() {
  sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Subscriber"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' "$1"
}

# Point a suite's WAL receiver at paradedb-ro, the CNPG read-only service. Enterprise runs a
# 3-instance cluster, so paradedb-ro routes to a standby streaming from paradedb-rw.
rewrite_wal_receiver() {
  sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "WalReceiver"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-ro:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' "$1"
}

# Point a logical-replication suite at its publisher and subscriber.
rewrite_pub_sub() {
  rewrite_publisher  "$1"
  rewrite_subscriber "$1"
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
  # A _phys topology adds a physical replica streaming from paradedb-rw: sub_phys is that
  # WAL sender/receiver pair on its own, pub_sub_phys hangs it off a logical subscriber.
  case "${topology}" in
    single)       rewrite_single     "${path}" ;;
    pub_sub)      rewrite_pub_sub    "${path}" ;;
    vanilla)      rewrite_vanilla    "${path}" ;;
    sub_phys)     rewrite_subscriber "${path}"; rewrite_wal_receiver "${path}" ;;
    pub_sub_phys) rewrite_pub_sub    "${path}"; rewrite_wal_receiver "${path}" ;;
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

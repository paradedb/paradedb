#!/bin/bash
#
# Shared setup for this template's first_ commands (and constants for the driver). Antithesis
# runs exactly one first_ per timeline, so each first_ points its suite at the cluster, builds
# the schema fault-free, and publishes the workload at WORKLOAD_LINK for the singleton_driver.

set -Eeuo pipefail

SUITE_DIR=/home/app/stressgres/suites
STRESSGRES=/symbols/stressgres
PARADEDB_ADMIN_CONN="postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5"

# The paired singleton_driver runs this symlink; each first_ repoints it at its own suite.
WORKLOAD_LINK=/tmp/stressgres-workload.toml

# Point a single-node suite at paradedb-rw. The connection-string query params are
# fail-fast timeouts so a dropped socket lands inside the reconnect-grace window.
rewrite_single() {
  sed -i 's|\[server\.style\.Automatic\]|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' "$1"
}

# Point a suite's publisher at the vanilla Postgres pod (an upstream primary we do not
# control).
rewrite_publisher() {
  sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Publisher"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@logical-replication-publisher:5432/postgres?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' "$1"
}

# Point the first remaining subscriber in a suite at a database on paradedb-rw. Repeated calls
# rewrite successive subscribers because each replacement removes its `postgresql_conf` marker.
rewrite_subscriber_database() {
  local path="$1" database="$2"
  sed -i -z "s|\[server\.style\.Automatic\]\npostgresql_conf = \"Subscriber\"|[server.style.With]\nconnection_string = \"postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/${database}?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15\"|" "${path}"
}

# Point a suite's subscriber at the default database on paradedb-rw.
rewrite_subscriber() {
  rewrite_subscriber_database "$1" paradedb
}

create_database_if_missing() {
  local database="$1"
  if [[ "$(psql "${PARADEDB_ADMIN_CONN}" -At -v ON_ERROR_STOP=1 -c "SELECT 1 FROM pg_database WHERE datname = '${database}'")" != "1" ]]; then
    psql "${PARADEDB_ADMIN_CONN}" -v ON_ERROR_STOP=1 -c "CREATE DATABASE \"${database}\""
  fi
}

# Point a suite's WAL receiver at paradedb-ro, the CNPG read-only service. Enterprise runs a
# 3-instance cluster, so paradedb-ro routes to a standby streaming from paradedb-rw.
rewrite_wal_receiver() {
  sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "WalReceiver"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-ro:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' "$1"
}

# Point a logical-replication suite at its publisher and subscriber.
rewrite_pub_sub() {
  rewrite_publisher "$1"
  rewrite_subscriber "$1"
}

# Give each logical subscriber its own database on the CNPG primary. They share the faulted
# PostgreSQL instance but maintain independent subscriptions, tables, indexes, and replay state.
rewrite_pub_multi_sub() {
  local path="$1"
  rewrite_publisher "${path}"
  create_database_if_missing stressgres_subscriber_a
  create_database_if_missing stressgres_subscriber_b
  rewrite_subscriber_database "${path}" stressgres_subscriber_a
  rewrite_subscriber_database "${path}" stressgres_subscriber_b
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
    single) rewrite_single "${path}" ;;
    pub_sub) rewrite_pub_sub "${path}" ;;
    pub_multi_sub) rewrite_pub_multi_sub "${path}" ;;
    sub_phys)
      rewrite_subscriber "${path}"
      rewrite_wal_receiver "${path}"
      ;;
    pub_sub_phys)
      rewrite_pub_sub "${path}"
      rewrite_wal_receiver "${path}"
      ;;
    *)
      echo "unknown topology: ${topology}" >&2
      exit 1
      ;;
  esac

  # antithesis-bootstrap-gate.yaml holds the fault-free bootstrap phase open until paradedb-rw
  # accepts connections, so no fixed wait is needed here.
  echo ""
  echo "Building schema for ${toml}..."
  "${STRESSGRES}" headless "${path}" --setup-only --reconnect-grace 200000

  # Publish the workload. Exactly one first_ runs per timeline, so the driver always runs
  # the suite we just built.
  ln -sf "${path}" "${WORKLOAD_LINK}"

  echo ""
  echo "Schema build complete!"
}

#!/bin/bash

# Runs before fault injection begins: point the suite at the cluster, build the schema
# with --setup-only, then exit. The paired singleton_driver runs the workload against it.

set -Eeuo pipefail

echo ""
echo "Pointing suite at the ParadeDB cluster..."
# connect_timeout/keepalive/tcp_user_timeout make a dropped socket fail fast, so a recovery
# poke lands inside the reconnect-grace window.
sed -i 's|postgresql://postgres:postgres@localhost:5432/postgres|postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15|g' /home/app/stressgres/suites/vanilla-postgres.toml

echo ""
echo "Waiting 60s for the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Building schema for vanilla-postgres.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/vanilla-postgres.toml --setup-only --reconnect-grace 3600000

echo ""
echo "Schema build complete!"

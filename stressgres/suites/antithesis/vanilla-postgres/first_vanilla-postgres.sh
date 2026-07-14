#!/bin/bash

set -Eeuo pipefail

# First command: runs fault-free, before any faults or driver commands. Rewrites the suite's
# connection string, then builds the schema with --setup-only and exits. The paired
# singleton_driver_ command runs the workload against it.
echo ""
echo "Updating suite to use Antithesis connection..."
# The connect_timeout/keepalive/tcp_user_timeout params make a dropped or partitioned socket fail fast, so a recovery poke lands inside the reconnect-grace window
sed -i 's|postgresql://postgres:postgres@localhost:5432/postgres|postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15|g' /home/app/stressgres/suites/vanilla-postgres.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Building schema for vanilla-postgres.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/vanilla-postgres.toml --setup-only --reconnect-grace 3600000

echo ""
echo "Schema build completed!"

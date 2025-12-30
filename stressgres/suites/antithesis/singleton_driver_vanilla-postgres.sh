#!/bin/bash

set -euo pipefail

# See the README.md about this connection string.
echo ""
echo "Updating suite default connection_string..."
sed -i 's|postgresql://postgres:postgres@localhost:5432/postgres|postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb|g' /home/app/suites/vanilla-postgres.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite vanilla-postgres.toml..."
# Run for 100 seconds: running for 10 minutes causes a "All commands were run to completion at least once" error in Antithesis.
/home/app/target/release/stressgres headless /home/app/suites/vanilla-postgres.toml --runtime 100000 --log-interval-ms 10000

echo ""
echo "Stressgres completed!"

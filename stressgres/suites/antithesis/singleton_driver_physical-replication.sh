#!/bin/bash

set -Eeuo pipefail

# See stressgres/README.md about these connection strings.
echo ""
echo "Updating suite default connection_string..."
sed -i 's|postgresql://postgres:postgres@localhost:5432/postgres|postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb|g' /home/app/suites/physical-replication.toml
sed -i 's|postgresql://postgres:postgres@localhost:5433/postgres|postgresql://postgres:antithesis-super-secret-password@paradedb-ro:5432/paradedb|g' /home/app/suites/physical-replication.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite physical-replication.toml..."
# Run for 100 seconds: running for 10 minutes causes a "All commands were run to completion at least once" error in Antithesis.
/home/app/target/release/stressgres headless /home/app/suites/physical-replication.toml --runtime 100000 --log-interval-ms 10000

echo ""
echo "Stressgres completed!"

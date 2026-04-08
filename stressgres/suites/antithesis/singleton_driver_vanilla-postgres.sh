#!/bin/bash

set -Eeuo pipefail

# See stressgres/README.md about this connection string.
echo ""
echo "Updating suite to use Antithesis connection..."
sed -i 's|postgresql://postgres:postgres@localhost:5432/postgres|postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb|g' /home/app/stressgres/suites/vanilla-postgres.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite vanilla-postgres.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/vanilla-postgres.toml --runtime 100000 --log-interval-ms 10000

echo ""
echo "Stressgres completed!"

#!/bin/bash

set -Eeuo pipefail

# See stressgres/README.md about this connection string.
echo ""
echo "Updating suite to use Antithesis connection..."
sed -i 's|\[server\.style\.Automatic\]|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb"|' /home/app/stressgres/suites/background-merge.toml
# The suite hardcodes the database name 'stressgres' in ALTER DATABASE — update it to match Antithesis
sed -i 's|ALTER DATABASE stressgres|ALTER DATABASE paradedb|g' /home/app/stressgres/suites/background-merge.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite background-merge.toml..."
# Run for 100 seconds: running for 10 minutes causes a "All commands were run to completion at least once" error in Antithesis.
/home/app/target/release/stressgres headless /home/app/stressgres/suites/background-merge.toml --runtime 100000 --log-interval-ms 10000

echo ""
echo "Stressgres completed!"

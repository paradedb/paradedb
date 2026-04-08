#!/bin/bash

set -Eeuo pipefail

# See stressgres/README.md about this connection string.
echo ""
echo "Updating suite to use Antithesis connection..."
sed -i 's|\[server\.style\.Automatic\]|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb"|' /home/app/stressgres/suites/single-server.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite single-server.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/single-server.toml --runtime 100000 --log-interval-ms 10000

echo ""
echo "Stressgres completed!"

#!/bin/bash

set -Eeuo pipefail

# See stressgres/README.md about this connection string.
echo ""
echo "Updating suite to use Antithesis connection..."
sed -i 's|\[server\.style\.Automatic\]|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb"|' /home/app/stressgres/suites/bulk-updates.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite bulk-updates.toml..."
# --reconnect-grace-ms is deliberately larger than --runtime so a transient DB fault
# (Antithesis stop/kill/partition) is tolerated for the whole run instead of failing it;
# the run ends on its own runtime. Keep grace > runtime if you change either.
/home/app/target/release/stressgres headless /home/app/stressgres/suites/bulk-updates.toml --runtime 100000 --log-interval-ms 10000 --reconnect-grace-ms 300000

echo ""
echo "Stressgres completed!"

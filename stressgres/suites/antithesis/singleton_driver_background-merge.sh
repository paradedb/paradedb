#!/bin/bash

set -Eeuo pipefail

# See stressgres/README.md about this connection string.
echo ""
echo "Updating suite to use Antithesis connection..."
sed -i 's|\[server\.style\.Automatic\]|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb"|' /home/app/stressgres/suites/background-merge.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite background-merge.toml..."
# reconnect-grace resets after each successful reconnect, so it only needs to exceed the
# longest single outage: a paradedb node kill + CloudNativePG pod recovery can take ~75s,
# so 180s (also > --runtime) leaves margin. Keep it above --runtime if that ever changes.
/home/app/target/release/stressgres headless /home/app/stressgres/suites/background-merge.toml --runtime 100000 --log-interval-ms 10000 --reconnect-grace-ms 180000

echo ""
echo "Stressgres completed!"

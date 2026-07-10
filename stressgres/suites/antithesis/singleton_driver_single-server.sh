#!/bin/bash

set -Eeuo pipefail

# See stressgres/README.md about this connection string.
echo ""
echo "Updating suite to use Antithesis connection..."
# The connect_timeout/keepalive/tcp_user_timeout params make a dropped or partitioned
# socket fail fast instead of hanging ~130s, so a recovery poke lands inside the window.
sed -i 's|\[server\.style\.Automatic\]|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_retries=3\&tcp_user_timeout=15"|' /home/app/stressgres/suites/single-server.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite single-server.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/single-server.toml --runtime 100000 --log-interval-ms 10000 --reconnect-grace 3600000 --reconnect-grace-file /tmp/stressgres-reconnect-grace

echo ""
echo "Stressgres completed!"

#!/bin/bash

# Fault-free setup: point at the cluster, build the schema (--setup-only), exit.

set -Eeuo pipefail

echo ""
echo "Pointing suite at the ParadeDB cluster..."
# Fail-fast timeouts so a dropped socket lands inside the reconnect-grace window.
sed -i 's|\[server\.style\.Automatic\]|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' /home/app/stressgres/suites/single-server.toml

echo ""
echo "Waiting 60s for the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Building schema for single-server.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/single-server.toml --setup-only --reconnect-grace 3600000

echo ""
echo "Schema build complete!"

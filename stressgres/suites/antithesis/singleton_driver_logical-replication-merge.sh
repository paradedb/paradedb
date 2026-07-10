#!/bin/bash

set -Eeuo pipefail

# See stressgres/README.md about these connection strings.
#
# Publisher  -> vanilla Postgres pod (see docker/manifests/antithesis-paradedb.yaml).
#               Mirrors a real-world logical-replication topology where the
#               upstream primary is not under our control.
# Subscriber -> paradedb-rw (primary of the CNPG cluster, has pg_search).
echo ""
echo "Updating suite to use Antithesis connections..."
# The connect_timeout/keepalive/tcp_user_timeout params make a dropped or partitioned socket fail fast, so a recovery poke lands inside the reconnect-grace window
sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Publisher"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@logical-replication-publisher:5432/postgres?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_retries=3\&tcp_user_timeout=15"|' /home/app/stressgres/suites/logical-replication-merge.toml
sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Subscriber"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_retries=3\&tcp_user_timeout=15"|' /home/app/stressgres/suites/logical-replication-merge.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite logical-replication-merge.toml..."
# Run for 100 seconds: running for 10 minutes causes a "All commands were run to completion at least once" error in Antithesis.
/home/app/target/release/stressgres headless /home/app/stressgres/suites/logical-replication-merge.toml --runtime 100000 --log-interval-ms 10000 --reconnect-grace 3600000 --reconnect-grace-file /tmp/stressgres-reconnect-grace

echo ""
echo "Stressgres completed!"

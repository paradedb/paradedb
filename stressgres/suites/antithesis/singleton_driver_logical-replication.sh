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
sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Publisher"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@logical-replication-publisher:5432/postgres"|' /home/app/stressgres/suites/logical-replication.toml
sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Subscriber"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb"|' /home/app/stressgres/suites/logical-replication.toml

echo ""
echo "Sleeping for 60 seconds to allow the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Running Stressgres with suite logical-replication.toml..."
# Run for 100 seconds: running for 10 minutes causes a "All commands were run to completion at least once" error in Antithesis.
# reconnect-grace resets after each successful reconnect, so it only needs to exceed the
# longest single outage: a paradedb node kill + CloudNativePG pod recovery can take ~75s,
# so 180s (also > --runtime) leaves margin. Keep it above --runtime if that ever changes.
/home/app/target/release/stressgres headless /home/app/stressgres/suites/logical-replication.toml --runtime 100000 --log-interval-ms 10000 --reconnect-grace-ms 180000

echo ""
echo "Stressgres completed!"

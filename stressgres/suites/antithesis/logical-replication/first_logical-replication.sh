#!/bin/bash

# Runs before fault injection begins: point both servers at their clusters, build the schema
# with --setup-only, then exit. The paired singleton_driver runs the workload against it.
#
# Publisher  -> vanilla Postgres pod, an upstream primary we do not control.
# Subscriber -> paradedb-rw (the CNPG primary, has pg_search).

set -Eeuo pipefail

echo ""
echo "Pointing publisher and subscriber at their clusters..."
# Fail-fast timeouts so a dropped socket lands inside the reconnect-grace window.
sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Publisher"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@logical-replication-publisher:5432/postgres?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' /home/app/stressgres/suites/logical-replication.toml
sed -i -z 's|\[server\.style\.Automatic\]\npostgresql_conf = "Subscriber"|[server.style.With]\nconnection_string = "postgresql://postgres:antithesis-super-secret-password@paradedb-rw:5432/paradedb?connect_timeout=5\&keepalives=1\&keepalives_idle=5\&keepalives_interval=2\&keepalives_count=3\&tcp_user_timeout=15"|' /home/app/stressgres/suites/logical-replication.toml

echo ""
echo "Waiting 60s for the ParadeDB cluster to initialize..."
sleep 60

echo ""
echo "Building schema for logical-replication.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/logical-replication.toml --setup-only --reconnect-grace 200000

echo ""
echo "Schema build complete!"

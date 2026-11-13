#!/usr/bin/env bash
# Prints the largest partition's share of the rows for eight tenants with shares
# 81%, 15.74%, 2.81% and 5 x 0.09%, by partition count and by tenant sort order.
# usage: leaves.sh <psql command with connection flags>
set -euo pipefail
PSQL=("$@" -X -q -At -v ON_ERROR_STOP=1)

# Each order gives the sort rank of the tenants, largest tenant first.
for order in "1,2,3,4,5,6,7,8" "2,1,3,4,5,6,7,8" "5,1,2,3,4,6,7,8" "8,1,2,3,4,5,6,7" "4,8,1,2,3,5,6,7"; do
  "${PSQL[@]}" <<SQL
SELECT setseed(0.7);
DROP TABLE IF EXISTS leaf_probe;
CREATE UNLOGGED TABLE leaf_probe AS
SELECT i AS id,
       format('%s-0000-4000-8000-000000000000', lpad(((ARRAY[$order])[slot])::text, 8, '0'))::uuid AS tenant_id
FROM (SELECT i, CASE WHEN r < 0.81 THEN 1 WHEN r < 0.9674 THEN 2 WHEN r < 0.9955 THEN 3
                     ELSE 4 + (i % 5) END AS slot
      FROM (SELECT i, random() AS r FROM generate_series(1, 400000) i) g) s;
SQL
  for k in 4 8 16; do
    "${PSQL[@]}" <<SQL
SET client_min_messages = error;
DROP INDEX IF EXISTS leaf_probe_idx;
CREATE INDEX leaf_probe_idx ON leaf_probe USING bm25 (id, tenant_id)
WITH (partition_by = 'tenant_id, ctid', target_segment_count = $k);
SELECT 'tenant ranks $order, $k partitions: largest ' ||
       round(100.0 * max(num_docs) / sum(num_docs), 1) || '% of the rows, ' || count(*) || ' segments'
FROM paradedb.index_info('leaf_probe_idx');
SQL
  done
done
"${PSQL[@]}" -c "DROP TABLE IF EXISTS leaf_probe"

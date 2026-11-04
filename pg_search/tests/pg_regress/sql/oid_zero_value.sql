-- OID 0 is a valid SQL value, not NULL. Every path that reads an oid column back
-- from the index must return it as 0, and a real NULL must stay NULL.

\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CREATE TABLE oid_zero (id bigint PRIMARY KEY, k oid NOT NULL, n oid);
INSERT INTO oid_zero VALUES (1, 0, 0), (2, 1, 1), (3, 0, NULL);
CREATE INDEX oid_zero_idx ON oid_zero USING paradedb (id, k, n);
ANALYZE oid_zero;

-- Tantivy aggregate backend: GROUP BY key
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT k, count(*) FROM oid_zero WHERE id @@@ pdb.all() GROUP BY k ORDER BY k;
SELECT k, count(*) FROM oid_zero WHERE id @@@ pdb.all() GROUP BY k ORDER BY k;

-- 0 and NULL are separate groups
SELECT n, count(*) FROM oid_zero WHERE id @@@ pdb.all() GROUP BY n ORDER BY n NULLS LAST;

-- DataFusion aggregate backend
SET paradedb.max_term_agg_buckets = 1;
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT k, count(*) FROM oid_zero WHERE id @@@ pdb.all() GROUP BY k ORDER BY k;
SELECT k, count(*) FROM oid_zero WHERE id @@@ pdb.all() GROUP BY k ORDER BY k;
SELECT n, count(*) FROM oid_zero WHERE id @@@ pdb.all() GROUP BY n ORDER BY n NULLS LAST;
RESET paradedb.max_term_agg_buckets;

-- Fast field base scan: a Postgres aggregate over it must see the zeros
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT min(k), max(k) FROM oid_zero WHERE id @@@ pdb.all();
SELECT min(k), max(k) FROM oid_zero WHERE id @@@ pdb.all();
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT k FROM oid_zero WHERE id @@@ pdb.all() ORDER BY k;
SELECT k FROM oid_zero WHERE id @@@ pdb.all() ORDER BY k;
SELECT n FROM oid_zero WHERE id @@@ pdb.all() ORDER BY n NULLS LAST;

-- Aggregate over a join
SET paradedb.enable_join_custom_scan = on;
CREATE TABLE oid_zero_other (id bigint PRIMARY KEY, label text);
INSERT INTO oid_zero_other VALUES (1, 'a'), (3, 'a');
CREATE INDEX oid_zero_other_idx ON oid_zero_other USING paradedb (id, (label::pdb.literal));
ANALYZE oid_zero_other;
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT z.k, count(*) FROM oid_zero z JOIN oid_zero_other o ON z.id = o.id
WHERE z.id @@@ pdb.all() AND o.label === 'a' GROUP BY z.k ORDER BY z.k;
SELECT z.k, count(*) FROM oid_zero z JOIN oid_zero_other o ON z.id = o.id
WHERE z.id @@@ pdb.all() AND o.label === 'a' GROUP BY z.k ORDER BY z.k;
RESET paradedb.enable_join_custom_scan;

-- Plain Postgres, for comparison
SET paradedb.enable_aggregate_custom_scan TO off;
SET paradedb.enable_custom_scan TO off;
SELECT k, count(*) FROM oid_zero GROUP BY k ORDER BY k;
RESET paradedb.enable_custom_scan;
RESET paradedb.enable_aggregate_custom_scan;

DROP TABLE oid_zero_other;
DROP TABLE oid_zero;

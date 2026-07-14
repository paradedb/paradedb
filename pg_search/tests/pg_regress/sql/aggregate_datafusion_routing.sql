-- =====================================================================
-- Routing between the Tantivy and DataFusion aggregate backends
-- =====================================================================
-- A single-table GROUP BY routes to DataFusion (no bucket cap) when the
-- estimated group count exceeds paradedb.max_term_agg_buckets, otherwise it
-- stays on the faster Tantivy path. The estimate comes from
-- estimate_num_groups seeded with the base relation's post-filter row count, so
-- it reflects both the column's n_distinct and the @@@ selectivity.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS routing_test;
CREATE TABLE routing_test (id bigint, cat text, sub text);

-- 50 distinct cat values, 2 distinct sub values, over 100k rows
INSERT INTO routing_test
SELECT g,
       'cat_' || lpad((g % 50)::text, 2, '0'),
       'sub_' || (g % 2)::text
FROM generate_series(1, 100000) g;

CREATE INDEX routing_test_idx ON routing_test
USING bm25 (id, (cat::pdb.literal), (sub::pdb.literal)) WITH (key_field='id');

ANALYZE routing_test;

SET paradedb.max_term_agg_buckets = 10;

-- 50 groups > cap: unbounded GROUP BY routes to DataFusion.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT cat, COUNT(*) FROM routing_test WHERE id @@@ pdb.all() GROUP BY cat;

-- ...and DataFusion returns every group (no truncation).
SELECT COUNT(*) AS groups FROM (
    SELECT cat FROM routing_test WHERE id @@@ pdb.all() GROUP BY cat
) s;

-- Single column bounded by LIMIT+OFFSET within the cap: stays on Tantivy.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT cat, COUNT(*) FROM routing_test WHERE id @@@ pdb.all()
GROUP BY cat ORDER BY cat LIMIT 5 OFFSET 3;

-- LIMIT+OFFSET beyond the cap: bounded pushdown unsafe, so DataFusion.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT cat, COUNT(*) FROM routing_test WHERE id @@@ pdb.all()
GROUP BY cat ORDER BY cat LIMIT 8 OFFSET 5;

-- Multiple grouping columns cannot use the bounded pushdown safely: DataFusion.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT cat, sub, COUNT(*) FROM routing_test WHERE id @@@ pdb.all()
GROUP BY cat, sub ORDER BY cat, sub LIMIT 5;

-- Selective filter: only a few rows match, so few groups are possible even
-- though cat has 50 distinct values overall — stays on the fast Tantivy path.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT cat, COUNT(*) FROM routing_test WHERE id @@@ '7' GROUP BY cat;

-- Low grouping cardinality (2 groups < cap): stays on Tantivy.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT sub, COUNT(*) FROM routing_test WHERE id @@@ pdb.all() GROUP BY sub;

RESET paradedb.max_term_agg_buckets;
DROP TABLE routing_test;

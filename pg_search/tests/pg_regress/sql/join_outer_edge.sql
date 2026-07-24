-- Outer-join edge cases through JoinScan: ORDER BY on the nullable side,
-- score on the nullable side, cross-table OR predicates, ON-clause routing,
-- and a FULL join that activates the custom scan. join_outer.sql covers the
-- basic LEFT / RIGHT shapes.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS oj_fact CASCADE;
DROP TABLE IF EXISTS oj_dim CASCADE;

CREATE TABLE oj_fact (id INT PRIMARY KEY, dim_id INT, txt TEXT);
CREATE TABLE oj_dim (id INT PRIMARY KEY, txt TEXT, price INT);

-- Every third oj_fact row has a NULL dim_id (never matches);
-- oj_dim rows 41..60 have no oj_fact partner.
INSERT INTO oj_fact
SELECT g, CASE WHEN g % 3 = 0 THEN NULL ELSE (g % 40) + 1 END, 'alpha item ' || g
FROM generate_series(1, 100) g;
INSERT INTO oj_dim
SELECT g, 'beta item ' || g, g * 10
FROM generate_series(1, 60) g;

CREATE INDEX oj_fact_idx ON oj_fact
USING bm25 (id, dim_id, txt)
WITH (key_field='id', numeric_fields='{"dim_id":{"fast":true}}', text_fields='{"txt":{"fast":true}}');
CREATE INDEX oj_dim_idx ON oj_dim
USING bm25 (id, txt, price)
WITH (key_field='id', numeric_fields='{"price":{"fast":true}}', text_fields='{"txt":{"fast":true}}');

ANALYZE oj_fact;
ANALYZE oj_dim;

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- ORDER BY on the nullable side: null-extended rows sort as a NULL key
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id WHERE a.txt @@@ 'alpha' ORDER BY b.id NULLS FIRST, a.id LIMIT 8;
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id WHERE a.txt @@@ 'alpha' ORDER BY b.id NULLS FIRST, a.id LIMIT 8;

-- =============================================================================
-- Score on the nullable side: NULL for null-extended rows
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT a.id, paradedb.score(b.id) FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id WHERE a.txt @@@ 'alpha' ORDER BY a.id LIMIT 8;
SELECT a.id, paradedb.score(b.id) FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id WHERE a.txt @@@ 'alpha' ORDER BY a.id LIMIT 8;

-- =============================================================================
-- Cross-table OR search predicate as a WHERE (post-join) filter
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id WHERE a.txt @@@ 'alpha' OR b.txt @@@ 'beta' ORDER BY a.id LIMIT 8;
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id WHERE a.txt @@@ 'alpha' OR b.txt @@@ 'beta' ORDER BY a.id LIMIT 8;

-- =============================================================================
-- ON-clause qual on the nullable side only: PG pushes it into that scan,
-- so JoinScan sees a plain equi-join and stays active
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id AND b.price > 100 WHERE a.txt @@@ 'alpha' ORDER BY a.id LIMIT 8;
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id AND b.price > 100 WHERE a.txt @@@ 'alpha' ORDER BY a.id LIMIT 8;

-- =============================================================================
-- Two-sided non-equi ON condition: declines with a warning
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id AND a.id < b.price WHERE a.txt @@@ 'alpha' ORDER BY a.id LIMIT 8;
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id AND a.id < b.price WHERE a.txt @@@ 'alpha' ORDER BY a.id LIMIT 8;

-- =============================================================================
-- FULL join: the search predicate lives inside a pulled-up subquery, below
-- the join, so PG can neither reduce the FULL join nor reject it, and
-- JoinScan runs it
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, b.id FROM (SELECT * FROM oj_fact WHERE txt @@@ 'alpha') f FULL JOIN oj_dim b ON f.dim_id = b.id ORDER BY f.id NULLS FIRST, b.id LIMIT 12;
SELECT f.id, b.id FROM (SELECT * FROM oj_fact WHERE txt @@@ 'alpha') f FULL JOIN oj_dim b ON f.dim_id = b.id ORDER BY f.id NULLS FIRST, b.id LIMIT 12;
SELECT COUNT(*) FROM (SELECT * FROM oj_fact WHERE txt @@@ 'alpha') f FULL JOIN oj_dim b ON f.dim_id = b.id;

-- =============================================================================
-- Baselines with JoinScan off: results must match the sections above.
-- paradedb.score() on the nullable side has no non-JoinScan fallback, so it
-- has no baseline here.
-- =============================================================================

SET paradedb.enable_join_custom_scan = off;

SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id WHERE a.txt @@@ 'alpha' ORDER BY b.id NULLS FIRST, a.id LIMIT 8;
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id WHERE a.txt @@@ 'alpha' OR b.txt @@@ 'beta' ORDER BY a.id LIMIT 8;
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id AND b.price > 100 WHERE a.txt @@@ 'alpha' ORDER BY a.id LIMIT 8;
SELECT a.id, b.id FROM oj_fact a LEFT JOIN oj_dim b ON a.dim_id = b.id AND a.id < b.price WHERE a.txt @@@ 'alpha' ORDER BY a.id LIMIT 8;
SELECT f.id, b.id FROM (SELECT * FROM oj_fact WHERE txt @@@ 'alpha') f FULL JOIN oj_dim b ON f.dim_id = b.id ORDER BY f.id NULLS FIRST, b.id LIMIT 12;
SELECT COUNT(*) FROM (SELECT * FROM oj_fact WHERE txt @@@ 'alpha') f FULL JOIN oj_dim b ON f.dim_id = b.id;

DROP TABLE oj_fact;
DROP TABLE oj_dim;

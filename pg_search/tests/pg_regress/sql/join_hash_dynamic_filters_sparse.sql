-- Regression coverage for sparse 1/2/3-column dynamic filter dispatch
-- (paradedb/paradedb#4895). Mirrors the production-side analogue of
-- tests/term_set_equivalence.rs::gallop_matches_linear_two_column_and_intersection
-- in tantivy: hash-join dynamic filters on sparsely-distributed (D = 1) FK
-- columns at column counts 1, 2, and 3.
--
-- Sizing (Option B from review):
--   t1: 30,000 rows, three independent FK columns (fk_a sorted, fk_b/fk_c
--   unsorted), all D = 1 within the corpus.
--   t2_a/b/c: 1,100 rows each (just over the FastField cardinality
--   threshold of 1024 so the FastFieldTermSetWeight dispatch path is
--   reached — below that, TermSetQuery routes to AutomatonWeight and
--   neither gallop nor smart-seek is exercised).
--   K/N for each pushed-down InList = 1100/30000 ≈ 0.0367. With
--   paradedb.term_set_gallop_max_density forced to 0.05 (vs the default
--   1/100 = 0.01), gallop is admitted on the sorted fk_a column.
--   LIMIT 10 on each query is required for the ParadeDB Join Scan custom
--   scan's cost model to pick it over a vanilla Hash Join (this also
--   matches the shape of the existing TEST 1/2 fixtures in join_hash.sql).
--
-- ---------------------------------------------------------------------------
-- A note on `dynamic_filter_pushdown=<strategy>` in the EXPLAIN output
-- ---------------------------------------------------------------------------
-- The strategy_sink is per-PgSearchScan and uses last-segment-wins semantics:
-- whatever the FINAL select_strategy() call wrote is what shows up. For
-- multi-column queries (Q2 / Q3) this means the displayed strategy reflects
-- the LAST filter dispatched, not the union of all filters. In particular,
-- a Q3 line reading `dynamic_filter_pushdown=linear` does NOT mean gallop
-- didn't fire — it means the last column dispatched landed on the linear
-- path (typically fk_b or fk_c, which are unsorted), overwriting the gallop
-- tag fk_a's dispatch wrote earlier. This is documented as Shape A in
-- implementation.md §7. The correctness gate (Block B) is what proves
-- gallop+smart-seek produced identical results to all-linear; the EXPLAIN
-- token is informational, not assertive.

-- Disable parallel workers so plans are deterministic.
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;
-- Force hash joins (the InList dynamic-filter pushdown path requires the
-- ParadeDB JoinScan custom scan, which composes with HashJoinExec).
SET enable_nestloop = OFF;
SET enable_mergejoin = OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_join_custom_scan = ON;
-- Admit gallop on K/N ≈ 0.0367. The default 1/100 = 0.01 would reject
-- this corpus; the override mirrors TEST 2b in join_hash.sql.
SET paradedb.term_set_gallop_max_density = 0.05;

-- ===========================================================================
-- Fixture
-- ===========================================================================

DROP TABLE IF EXISTS t1 CASCADE;
DROP TABLE IF EXISTS t2_a CASCADE;
DROP TABLE IF EXISTS t2_b CASCADE;
DROP TABLE IF EXISTS t2_c CASCADE;

-- t1: 30,000 rows. `body` carries a space-separated string ('doc 1',
-- 'doc 2', …) so the default BM25 tokenizer splits cleanly and the
-- shared 'doc' token lets `@@@ 'doc'` match every row. Each FK column
-- produces a distinct value per row (D = 1) by multiplying the row index
-- by a prime coprime to the modulus. Three different primes (7919, 6151,
-- 4099) give three independent pseudo-uniform spreads.
CREATE TABLE t1 (
    id    INTEGER PRIMARY KEY,
    body  TEXT,
    fk_a  BIGINT,
    fk_b  BIGINT,
    fk_c  BIGINT
);
INSERT INTO t1
SELECT
    i,
    'doc ' || i::text,
    (i * 7919 % 100000)::bigint,
    (i * 6151 % 100000)::bigint,
    (i * 4099 % 100000)::bigint
FROM generate_series(1, 30000) AS i;

-- Each t2_X provides 1,100 distinct fk values that are subsets of the
-- corresponding column in t1. Different `i` ranges per table so the
-- three filters intersect to a non-trivial nested result on the full
-- (unlimited) join:
--   Q1 (t2_a): t1.id ∈ [1, 1100]
--   Q2 (∩ t2_b): t1.id ∈ [1, 1100] ∩ [500, 1599] = [500, 1100]
--   Q3 (∩ t2_c): ∩ [800, 1899] = [800, 1100]
-- LIMIT 10 (with ORDER BY t1.id) yields a deterministic top-50 from
-- each.
-- t2_X tables also have BM25 indexes so the ParadeDB JoinScan custom
-- scan can engage on both sides of the join. (The Join Scan refuses to
-- engage if either side isn't BM25-indexed; without it the planner falls
-- back to vanilla Hash Join which doesn't run the dynamic-filter
-- pushdown machinery.)
CREATE TABLE t2_a (id INTEGER PRIMARY KEY, fk BIGINT, body TEXT);
INSERT INTO t2_a
SELECT i, (i * 7919 % 100000)::bigint, 'doc ' || i::text
FROM generate_series(1, 1100) AS i;

CREATE TABLE t2_b (id INTEGER PRIMARY KEY, fk BIGINT, body TEXT);
INSERT INTO t2_b
SELECT i, (i * 6151 % 100000)::bigint, 'doc ' || i::text
FROM generate_series(500, 1599) AS i;

CREATE TABLE t2_c (id INTEGER PRIMARY KEY, fk BIGINT, body TEXT);
INSERT INTO t2_c
SELECT i, (i * 4099 % 100000)::bigint, 'doc ' || i::text
FROM generate_series(800, 1899) AS i;

-- BM25 index on t1: id is the integer key, body is the text-searched
-- column, fk_a/fk_b/fk_c are fast numeric fields, segment is sorted ASC
-- by fk_a. Only fk_a is gallop-eligible; fk_b and fk_c land on the
-- linear-scan TermSetDocSet path (with smart-seek for forward-cursor
-- advancement).
CREATE INDEX t1_idx ON t1
USING bm25 (id, body, fk_a, fk_b, fk_c)
WITH (
    key_field = 'id',
    text_fields = '{"body": {"fast": true}}',
    numeric_fields = '{"fk_a": {"fast": true}, "fk_b": {"fast": true}, "fk_c": {"fast": true}}',
    sort_by = 'fk_a ASC NULLS FIRST'
);

CREATE INDEX t2_a_idx ON t2_a USING bm25 (id, fk, body)
WITH (key_field = 'id', numeric_fields = '{"fk": {"fast": true}}');

CREATE INDEX t2_b_idx ON t2_b USING bm25 (id, fk, body)
WITH (key_field = 'id', numeric_fields = '{"fk": {"fast": true}}');

CREATE INDEX t2_c_idx ON t2_c USING bm25 (id, fk, body)
WITH (key_field = 'id', numeric_fields = '{"fk": {"fast": true}}');

ANALYZE t1;
ANALYZE t2_a;
ANALYZE t2_b;
ANALYZE t2_c;

-- ===========================================================================
-- Q1: single-column filter (fk_a, the sorted/gallop-eligible column)
-- ===========================================================================

-- Block A: confirm pushdown shape. Look for `dynamic_filters=1,
-- dynamic_filter_pushdown=<strategy>` on the inner PgSearchScan; the
-- strategy value is captured but not asserted on (last-writer-wins; Q1 has
-- only one filter so the value is whatever fk_a's dispatch produced).
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.id
FROM t1
JOIN t2_a ON t1.fk_a = t2_a.fk
WHERE t1.body @@@ 'doc'
ORDER BY t1.id
LIMIT 10;

-- Block B: correctness. Run the same query with gallop disabled and
-- enabled, materialize each result into a TEMP TABLE, then compare row
-- counts and assert the symmetric difference is empty.

SET paradedb.term_set_gallop_enabled = OFF;
DROP TABLE IF EXISTS result_q1_linear;
CREATE TEMP TABLE result_q1_linear AS
SELECT t1.id
FROM t1
JOIN t2_a ON t1.fk_a = t2_a.fk
WHERE t1.body @@@ 'doc'
ORDER BY t1.id
LIMIT 10;

SET paradedb.term_set_gallop_enabled = ON;
DROP TABLE IF EXISTS result_q1_gallop;
CREATE TEMP TABLE result_q1_gallop AS
SELECT t1.id
FROM t1
JOIN t2_a ON t1.fk_a = t2_a.fk
WHERE t1.body @@@ 'doc'
ORDER BY t1.id
LIMIT 10;

SELECT
    (SELECT count(*) FROM result_q1_linear) AS linear_count,
    (SELECT count(*) FROM result_q1_gallop) AS gallop_count,
    (SELECT count(*) FROM result_q1_linear) = (SELECT count(*) FROM result_q1_gallop) AS counts_match;

SELECT 'q1 diff (must be empty)' AS check, source, id
FROM (
    SELECT 'linear-only' AS source, id FROM result_q1_linear
    EXCEPT
    SELECT 'linear-only', id FROM result_q1_gallop
    UNION ALL
    SELECT 'gallop-only' AS source, id FROM result_q1_gallop
    EXCEPT
    SELECT 'gallop-only', id FROM result_q1_linear
) AS diff
ORDER BY source, id;

DROP TABLE result_q1_linear;
DROP TABLE result_q1_gallop;

-- ===========================================================================
-- Q2: two-column filter (fk_a sorted + fk_b unsorted)
-- ===========================================================================

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.id
FROM t1
JOIN t2_a ON t1.fk_a = t2_a.fk
JOIN t2_b ON t1.fk_b = t2_b.fk
WHERE t1.body @@@ 'doc'
ORDER BY t1.id
LIMIT 10;

SET paradedb.term_set_gallop_enabled = OFF;
DROP TABLE IF EXISTS result_q2_linear;
CREATE TEMP TABLE result_q2_linear AS
SELECT t1.id
FROM t1
JOIN t2_a ON t1.fk_a = t2_a.fk
JOIN t2_b ON t1.fk_b = t2_b.fk
WHERE t1.body @@@ 'doc'
ORDER BY t1.id
LIMIT 10;

SET paradedb.term_set_gallop_enabled = ON;
DROP TABLE IF EXISTS result_q2_gallop;
CREATE TEMP TABLE result_q2_gallop AS
SELECT t1.id
FROM t1
JOIN t2_a ON t1.fk_a = t2_a.fk
JOIN t2_b ON t1.fk_b = t2_b.fk
WHERE t1.body @@@ 'doc'
ORDER BY t1.id
LIMIT 10;

SELECT
    (SELECT count(*) FROM result_q2_linear) AS linear_count,
    (SELECT count(*) FROM result_q2_gallop) AS gallop_count,
    (SELECT count(*) FROM result_q2_linear) = (SELECT count(*) FROM result_q2_gallop) AS counts_match;

SELECT 'q2 diff (must be empty)' AS check, source, id
FROM (
    SELECT 'linear-only' AS source, id FROM result_q2_linear
    EXCEPT
    SELECT 'linear-only', id FROM result_q2_gallop
    UNION ALL
    SELECT 'gallop-only' AS source, id FROM result_q2_gallop
    EXCEPT
    SELECT 'gallop-only', id FROM result_q2_linear
) AS diff
ORDER BY source, id;

DROP TABLE result_q2_linear;
DROP TABLE result_q2_gallop;

-- ===========================================================================
-- Q3: three-column filter (fk_a sorted + fk_b unsorted + fk_c unsorted)
-- ===========================================================================

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.id
FROM t1
JOIN t2_a ON t1.fk_a = t2_a.fk
JOIN t2_b ON t1.fk_b = t2_b.fk
JOIN t2_c ON t1.fk_c = t2_c.fk
WHERE t1.body @@@ 'doc'
ORDER BY t1.id
LIMIT 10;

SET paradedb.term_set_gallop_enabled = OFF;
DROP TABLE IF EXISTS result_q3_linear;
CREATE TEMP TABLE result_q3_linear AS
SELECT t1.id
FROM t1
JOIN t2_a ON t1.fk_a = t2_a.fk
JOIN t2_b ON t1.fk_b = t2_b.fk
JOIN t2_c ON t1.fk_c = t2_c.fk
WHERE t1.body @@@ 'doc'
ORDER BY t1.id
LIMIT 10;

SET paradedb.term_set_gallop_enabled = ON;
DROP TABLE IF EXISTS result_q3_gallop;
CREATE TEMP TABLE result_q3_gallop AS
SELECT t1.id
FROM t1
JOIN t2_a ON t1.fk_a = t2_a.fk
JOIN t2_b ON t1.fk_b = t2_b.fk
JOIN t2_c ON t1.fk_c = t2_c.fk
WHERE t1.body @@@ 'doc'
ORDER BY t1.id
LIMIT 10;

SELECT
    (SELECT count(*) FROM result_q3_linear) AS linear_count,
    (SELECT count(*) FROM result_q3_gallop) AS gallop_count,
    (SELECT count(*) FROM result_q3_linear) = (SELECT count(*) FROM result_q3_gallop) AS counts_match;

SELECT 'q3 diff (must be empty)' AS check, source, id
FROM (
    SELECT 'linear-only' AS source, id FROM result_q3_linear
    EXCEPT
    SELECT 'linear-only', id FROM result_q3_gallop
    UNION ALL
    SELECT 'gallop-only' AS source, id FROM result_q3_gallop
    EXCEPT
    SELECT 'gallop-only', id FROM result_q3_linear
) AS diff
ORDER BY source, id;

DROP TABLE result_q3_linear;
DROP TABLE result_q3_gallop;

-- ===========================================================================
-- Cleanup
-- ===========================================================================

DROP TABLE IF EXISTS t1 CASCADE;
DROP TABLE IF EXISTS t2_a CASCADE;
DROP TABLE IF EXISTS t2_b CASCADE;
DROP TABLE IF EXISTS t2_c CASCADE;

RESET paradedb.term_set_gallop_enabled;
RESET paradedb.term_set_gallop_max_density;
RESET paradedb.enable_join_custom_scan;
RESET enable_nestloop;
RESET enable_mergejoin;
RESET enable_indexscan;
RESET max_parallel_workers_per_gather;

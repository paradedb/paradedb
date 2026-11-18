-- Test for the JoinScan Custom Scan planning
-- HashJoin mechanics and metrics via EXPLAIN ANALYZE

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Make sure the GUC is enabled
SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: Hash Join
-- =============================================================================

DROP TABLE IF EXISTS hash_t1 CASCADE;
DROP TABLE IF EXISTS hash_t2 CASCADE;

CREATE TABLE hash_t1 (id INTEGER PRIMARY KEY, val TEXT) WITH (autovacuum_enabled = false);
CREATE TABLE hash_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT) WITH (autovacuum_enabled = false);

INSERT INTO hash_t1 SELECT i, 'val ' || i FROM generate_series(1, 1000) i;
INSERT INTO hash_t2 SELECT i, (i % 1000) + 1, 'val ' || i FROM generate_series(1, 1000) i;

CREATE INDEX hash_t1_idx ON hash_t1 USING paradedb (id, (val::pdb.unicode_words('columnar=true')));

CREATE INDEX hash_t2_idx ON hash_t2 USING paradedb (id, t1_id, val);

ANALYZE hash_t1;
ANALYZE hash_t2;

-- EXPLAIN ANALYZE to show HashJoin metrics.
-- Check in particular for the `dynamic_filter_pushdown=<strategy>` token (one of
-- {gallop, linear, bitset_from_postings, automaton, empty}; falls back to `true`
-- on the rare path where pushdown was indicated but no strategy tag was
-- recorded), which signals dynamic filters were pushed down into the Tantivy
-- query and reports the dispatched TermSet strategy.
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM hash_t1 t1
JOIN hash_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val ||| 'val'
ORDER BY t1.id ASC
LIMIT 10;

SELECT t1.val, t2.val
FROM hash_t1 t1
JOIN hash_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val ||| 'val'
ORDER BY t1.id ASC
LIMIT 10;

-- =============================================================================
-- TEST 2: Hash Join with sorted segment on the FK column (issue #4895)
-- =============================================================================
-- Purpose: capture an EXPLAIN ANALYZE of a hash-join probe where the inner
-- index is sorted ASC by the foreign-key column, demonstrating that the
-- dynamic_filter_pushdown=... EXPLAIN token reports the chosen strategy.
--
-- The test asserts BOTH dispatch outcomes:
--   2a. With paradedb.term_set_gallop_enabled = OFF, gallop is rejected
--       via the kill-switch. K/N = 0.75 is also above both bitset
--       density gates, so the planner falls through to LinearScan →
--       EXPLAIN says dynamic_filter_pushdown=linear.
--   2b. With paradedb.term_set_gallop_enabled at its default ON, gallop
--       fires on this sorted segment → EXPLAIN says
--       dynamic_filter_pushdown=gallop.

DROP TABLE IF EXISTS hash_sorted_t1 CASCADE;
DROP TABLE IF EXISTS hash_sorted_t2 CASCADE;

CREATE TABLE hash_sorted_t1 (id INTEGER PRIMARY KEY, val TEXT) WITH (autovacuum_enabled = false);
CREATE TABLE hash_sorted_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT) WITH (autovacuum_enabled = false);

-- t1: 1500 rows (clears the TermSetQuery FastField cardinality threshold of
-- 1024 so the FastField dispatch path is reached).
-- t2: 2000 rows with t1_id repeating across 1..1500 — K/N = 1500/2000 = 0.75.
-- TEST 2a flips `term_set_gallop_enabled` off to reject gallop via the
-- kill-switch; TEST 2b leaves the default on so gallop fires.
INSERT INTO hash_sorted_t1 SELECT i, 'val ' || i FROM generate_series(1, 1500) i;
INSERT INTO hash_sorted_t2 SELECT i, ((i - 1) % 1500) + 1, 'val ' || i FROM generate_series(1, 2000) i;

CREATE INDEX hash_sorted_t1_idx ON hash_sorted_t1 USING paradedb (id, (val::pdb.unicode_words('columnar=true')));

CREATE INDEX hash_sorted_t2_idx ON hash_sorted_t2 USING paradedb (id, t1_id, val) WITH (sort_by = 't1_id ASC NULLS FIRST');

ANALYZE hash_sorted_t1;
ANALYZE hash_sorted_t2;

-- TEST 2a: gallop kill-switch engaged. K/N = 0.75 is also above both
-- bitset gates (1/2000 and 1/200), so dispatch falls through to
-- LinearScan.
SET paradedb.term_set_gallop_enabled = OFF;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val ||| 'val'
ORDER BY t1.id ASC
LIMIT 10;

SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val ||| 'val'
ORDER BY t1.id ASC
LIMIT 10;

RESET paradedb.term_set_gallop_enabled;

-- TEST 2b: default behavior. Same data, same query — `gallop_enabled`
-- is ON by default, sorted segment + matching field → gallop fires.

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val ||| 'val'
ORDER BY t1.id ASC
LIMIT 10;

SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val ||| 'val'
ORDER BY t1.id ASC
LIMIT 10;

-- =============================================================================
-- TEST 3: paradedb.hash_join_inlist_pushdown_max_distinct_values = 0 forces
--         HashTableLookupExpr pushdown (dynamic_filter_pushdown_hash_*)
-- =============================================================================
-- Setting `hash_join_inlist_pushdown_max_distinct_values = 0` causes DataFusion's
-- HashJoinExec (`num_of_distinct_key > max_distinct_values`) to emit a
-- `HashTableLookupExpr` instead of an `InListExpr`. That `HashTableLookupExpr`
-- is pushed down into Tantivy via `TermPredicateQuery` and surfaces as
-- `dynamic_filter_pushdown_hash_<strategy>=1` in EXPLAIN ANALYZE.
SET paradedb.hash_join_inlist_pushdown_max_distinct_values = 0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val ||| 'val'
ORDER BY t1.id ASC
LIMIT 10;

SELECT t1.val, t2.val
FROM hash_sorted_t1 t1
JOIN hash_sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val ||| 'val'
ORDER BY t1.id ASC
LIMIT 10;

-- =============================================================================
-- TEST 4: HashTableLookupExpr pushdown on dictionary-encoded string keys
--         (sorted gallop, selective bitset_from_postings, dense linear, and
--          selective probe filter)
-- =============================================================================

DROP TABLE IF EXISTS hash_str_build CASCADE;
DROP TABLE IF EXISTS hash_str_probe_sorted CASCADE;
DROP TABLE IF EXISTS hash_str_probe_unsorted CASCADE;

CREATE TABLE hash_str_build (
    id   INTEGER PRIMARY KEY,
    k    TEXT NOT NULL,
    body TEXT NOT NULL
) WITH (autovacuum_enabled = false);

-- Mix short (<= 12 bytes, inlined Utf8View) and long (> 12 bytes, buffer-backed
-- Utf8View) keys to exercise both StringViewArray hash representations.
INSERT INTO hash_str_build
SELECT i,
       CASE WHEN i % 2 = 0 THEN 's' || i::text
            ELSE 'long-join-key-value-' || lpad(i::text, 6, '0')
       END,
       'doc ' || i::text
FROM generate_series(1, 500) i;

CREATE TABLE hash_str_probe_sorted (
    id   INTEGER PRIMARY KEY,
    fk   TEXT NOT NULL,
    tag  TEXT NOT NULL
) WITH (autovacuum_enabled = false);

INSERT INTO hash_str_probe_sorted
SELECT i,
       CASE WHEN ((i - 1) % 2500 + 1) % 2 = 0 THEN 's' || ((i - 1) % 2500 + 1)::text
            ELSE 'long-join-key-value-' || lpad(((i - 1) % 2500 + 1)::text, 6, '0')
       END,
       CASE WHEN i IN (7, 507) THEN 'rare' ELSE 'common' END
FROM generate_series(1, 5000) i;

CREATE TABLE hash_str_probe_unsorted (
    id      INTEGER PRIMARY KEY,
    fk_lit  TEXT NOT NULL,
    fk_tok  TEXT NOT NULL,
    tag     TEXT NOT NULL
) WITH (autovacuum_enabled = false);

INSERT INTO hash_str_probe_unsorted
SELECT i,
       CASE WHEN ((i - 1) % 2500 + 1) % 2 = 0 THEN 's' || ((i - 1) % 2500 + 1)::text
            ELSE 'long-join-key-value-' || lpad(((i - 1) % 2500 + 1)::text, 6, '0')
       END,
       CASE WHEN ((i - 1) % 2500 + 1) % 2 = 0 THEN 's' || ((i - 1) % 2500 + 1)::text
            ELSE 'long-join-key-value-' || lpad(((i - 1) % 2500 + 1)::text, 6, '0')
       END,
       CASE WHEN i IN (7, 507) THEN 'rare' ELSE 'common' END
FROM generate_series(1, 5000) i;

CREATE INDEX hash_str_build_idx ON hash_str_build
USING paradedb (id, (k::pdb.literal), (body::pdb.unicode_words('columnar=true')));

CREATE INDEX hash_str_probe_sorted_idx ON hash_str_probe_sorted
USING paradedb (id, (fk::pdb.literal), (tag::pdb.literal))
WITH (sort_by = 'fk ASC NULLS FIRST');

CREATE INDEX hash_str_probe_unsorted_idx ON hash_str_probe_unsorted
USING paradedb (id, (fk_lit::pdb.literal), (fk_tok::pdb.unicode_words('columnar=true')), (tag::pdb.literal));

ANALYZE hash_str_build;
ANALYZE hash_str_probe_sorted;
ANALYZE hash_str_probe_unsorted;

-- 4a: Sorted literal column -> dynamic_filter_pushdown_hash_gallop
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT b.id, p.id
FROM hash_str_build b
JOIN hash_str_probe_sorted p ON b.k = p.fk
WHERE b.body ||| 'doc' AND b.id <= 200
ORDER BY b.id ASC, p.id ASC
LIMIT 10;

-- 4b: Unsorted literal column, highly selective (K=5, N=5000, D=2 -> 5/5000 <= 0.005)
--     -> dynamic_filter_pushdown_hash_bitset_from_postings
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT b.id, p.id
FROM hash_str_build b
JOIN hash_str_probe_unsorted p ON b.k = p.fk_lit
WHERE b.body ||| 'doc' AND b.id <= 5
ORDER BY b.id ASC, p.id ASC
LIMIT 10;

-- 4c: Unsorted tokenized (columnar=true, non-keyword) text column -> fast-field
--     dictionary ordinal bitset (dynamic_filter_pushdown_hash_linear)
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT b.id, p.id
FROM hash_str_build b
JOIN hash_str_probe_unsorted p ON b.k = p.fk_tok
WHERE b.body ||| 'doc' AND b.id <= 200
ORDER BY b.id ASC, p.id ASC
LIMIT 10;

-- 4d: Combined with a very selective probe-side filter (`p.tag = 'rare'`, 2 rows)
SELECT b.id, p.id
FROM hash_str_build b
JOIN hash_str_probe_unsorted p ON b.k = p.fk_lit
WHERE b.body ||| 'doc' AND p.tag = 'rare'
ORDER BY b.id ASC, p.id ASC
LIMIT 10;

-- 4e: Equivalence check between HashTableLookupExpr (max_distinct=0) and InList
CREATE TEMP TABLE res_hash AS
SELECT b.id AS bid, p.id AS pid
FROM hash_str_build b
JOIN hash_str_probe_unsorted p ON b.k = p.fk_lit
WHERE b.body ||| 'doc' AND b.id <= 200
ORDER BY b.id ASC, p.id ASC
LIMIT 100;

RESET paradedb.hash_join_inlist_pushdown_max_distinct_values;

CREATE TEMP TABLE res_inlist AS
SELECT b.id AS bid, p.id AS pid
FROM hash_str_build b
JOIN hash_str_probe_unsorted p ON b.k = p.fk_lit
WHERE b.body ||| 'doc' AND b.id <= 200
ORDER BY b.id ASC, p.id ASC
LIMIT 100;

SELECT count(*) AS diff_rows FROM (
    (SELECT * FROM res_hash EXCEPT ALL SELECT * FROM res_inlist)
    UNION ALL
    (SELECT * FROM res_inlist EXCEPT ALL SELECT * FROM res_hash)
) d;

DROP TABLE res_hash;
DROP TABLE res_inlist;
DROP TABLE IF EXISTS hash_str_build CASCADE;
DROP TABLE IF EXISTS hash_str_probe_sorted CASCADE;
DROP TABLE IF EXISTS hash_str_probe_unsorted CASCADE;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS hash_t1 CASCADE;
DROP TABLE IF EXISTS hash_t2 CASCADE;
DROP TABLE IF EXISTS hash_sorted_t1 CASCADE;
DROP TABLE IF EXISTS hash_sorted_t2 CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;

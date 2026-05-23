-- Cross-strategy equivalence tests for the TermSet dispatch decision.
--
-- For each pair of strategies (bitset vs linear, gallop vs linear), this
-- file runs the same query under each strategy by toggling the
-- paradedb.term_set_*_max_density GUCs, captures the result into a temp
-- table, and asserts the symmetric difference is empty via EXCEPT.
--
-- Mirrors the gallop_matches_linear pattern from join_hash_dynamic_filters_sparse.sql.
-- This is the SQL-level analog of the tantivy-side proptest
-- `bitset_equivalent_to_linear_scan` — the proptest covers algorithmic
-- correctness with randomized inputs, this file pins the equivalence on
-- representative production-shaped corpora.

-- Plan-stabilizing GUCs.
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;
SET enable_nestloop = OFF;
SET enable_mergejoin = OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_join_custom_scan = ON;

-- ============================================================================
-- Fixtures
-- ============================================================================
-- Same shapes as term_set_dispatch.sql. K is controlled per-test via the
-- `ts_outer.id <= K` predicate.

DROP TABLE IF EXISTS ts_outer CASCADE;
DROP TABLE IF EXISTS ts_unique CASCADE;
DROP TABLE IF EXISTS ts_multi CASCADE;
DROP TABLE IF EXISTS ts_sorted CASCADE;

CREATE TABLE ts_outer (id INTEGER PRIMARY KEY, val TEXT);
INSERT INTO ts_outer SELECT i, 'doc' FROM generate_series(1, 200) i;
CREATE INDEX ts_outer_idx ON ts_outer USING bm25 (id, val)
WITH (key_field = 'id', text_fields = '{"val": {"fast": true}}');

CREATE TABLE ts_unique (id INTEGER PRIMARY KEY, fk INTEGER, val TEXT);
INSERT INTO ts_unique SELECT i, i, 'doc' FROM generate_series(1, 10000) i;
CREATE INDEX ts_unique_idx ON ts_unique USING bm25 (id, fk, val)
WITH (key_field = 'id', numeric_fields = '{"fk": {"fast": true}}');

CREATE TABLE ts_multi (id INTEGER PRIMARY KEY, fk INTEGER, val TEXT);
INSERT INTO ts_multi SELECT i, ((i - 1) % 100) + 1, 'doc' FROM generate_series(1, 10000) i;
CREATE INDEX ts_multi_idx ON ts_multi USING bm25 (id, fk, val)
WITH (key_field = 'id', numeric_fields = '{"fk": {"fast": true}}');

CREATE TABLE ts_sorted (id INTEGER PRIMARY KEY, fk INTEGER, val TEXT);
INSERT INTO ts_sorted SELECT i, i, 'doc' FROM generate_series(1, 10000) i;
CREATE INDEX ts_sorted_idx ON ts_sorted USING bm25 (id, fk, val)
WITH (
    key_field = 'id',
    numeric_fields = '{"fk": {"fast": true}}',
    sort_by = 'fk ASC NULLS FIRST'
);

ANALYZE ts_outer;
ANALYZE ts_unique;
ANALYZE ts_multi;
ANALYZE ts_sorted;

-- ============================================================================
-- Q1: bitset vs linear on a D=1 (unique) column
-- ============================================================================
-- Same join, same K=100, same expected row set. Force bitset by widening
-- the unique gate, then force linear by closing it. The EXCEPT-based diff
-- below must be empty.

SET paradedb.term_set_bitset_max_density_unique = 1.0;
DROP TABLE IF EXISTS result_q1_bitset;
CREATE TEMP TABLE result_q1_bitset AS
SELECT ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 100
ORDER BY ts_unique.id;
RESET paradedb.term_set_bitset_max_density_unique;

SET paradedb.term_set_bitset_max_density_unique = 0.0;
DROP TABLE IF EXISTS result_q1_linear;
CREATE TEMP TABLE result_q1_linear AS
SELECT ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 100
ORDER BY ts_unique.id;
RESET paradedb.term_set_bitset_max_density_unique;

SELECT
    (SELECT count(*) FROM result_q1_bitset) AS bitset_count,
    (SELECT count(*) FROM result_q1_linear) AS linear_count,
    (SELECT count(*) FROM result_q1_bitset) = (SELECT count(*) FROM result_q1_linear) AS counts_match;

SELECT 'q1 diff (must be empty)' AS check, source, id
FROM (
    SELECT 'bitset-only' AS source, id FROM result_q1_bitset
    EXCEPT
    SELECT 'bitset-only', id FROM result_q1_linear
    UNION ALL
    SELECT 'linear-only' AS source, id FROM result_q1_linear
    EXCEPT
    SELECT 'linear-only', id FROM result_q1_bitset
) AS diff
ORDER BY source, id;

DROP TABLE result_q1_bitset;
DROP TABLE result_q1_linear;

-- ============================================================================
-- Q2: bitset vs linear on a D=100 (multi) column
-- ============================================================================
-- Same shape as Q1 but on ts_multi. Higher K (200) so the multi gate is
-- exercised on both sides of its threshold.

SET paradedb.term_set_bitset_max_density_multi = 1.0;
DROP TABLE IF EXISTS result_q2_bitset;
CREATE TEMP TABLE result_q2_bitset AS
SELECT ts_multi.id
FROM ts_outer JOIN ts_multi ON ts_outer.id = ts_multi.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 200
ORDER BY ts_multi.id;
RESET paradedb.term_set_bitset_max_density_multi;

SET paradedb.term_set_bitset_max_density_multi = 0.0;
DROP TABLE IF EXISTS result_q2_linear;
CREATE TEMP TABLE result_q2_linear AS
SELECT ts_multi.id
FROM ts_outer JOIN ts_multi ON ts_outer.id = ts_multi.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 200
ORDER BY ts_multi.id;
RESET paradedb.term_set_bitset_max_density_multi;

SELECT
    (SELECT count(*) FROM result_q2_bitset) AS bitset_count,
    (SELECT count(*) FROM result_q2_linear) AS linear_count,
    (SELECT count(*) FROM result_q2_bitset) = (SELECT count(*) FROM result_q2_linear) AS counts_match;

SELECT 'q2 diff (must be empty)' AS check, source, id
FROM (
    SELECT 'bitset-only' AS source, id FROM result_q2_bitset
    EXCEPT
    SELECT 'bitset-only', id FROM result_q2_linear
    UNION ALL
    SELECT 'linear-only' AS source, id FROM result_q2_linear
    EXCEPT
    SELECT 'linear-only', id FROM result_q2_bitset
) AS diff
ORDER BY source, id;

DROP TABLE result_q2_bitset;
DROP TABLE result_q2_linear;

-- ============================================================================
-- Q3: gallop vs linear on a sorted segment
-- ============================================================================
-- ts_sorted has sort_by=fk ASC so gallop is admissible. Compare gallop
-- (default gallop_enabled=on) against linear (gallop disabled + both
-- bitset gates closed, forcing fall-through to LinearScan).

DROP TABLE IF EXISTS result_q3_gallop;
CREATE TEMP TABLE result_q3_gallop AS
SELECT ts_sorted.id
FROM ts_outer JOIN ts_sorted ON ts_outer.id = ts_sorted.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 100
ORDER BY ts_sorted.id;

SET paradedb.term_set_gallop_enabled = OFF;
SET paradedb.term_set_bitset_max_density_unique = 0.0;
SET paradedb.term_set_bitset_max_density_multi = 0.0;
DROP TABLE IF EXISTS result_q3_linear;
CREATE TEMP TABLE result_q3_linear AS
SELECT ts_sorted.id
FROM ts_outer JOIN ts_sorted ON ts_outer.id = ts_sorted.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 100
ORDER BY ts_sorted.id;
RESET paradedb.term_set_gallop_enabled;
RESET paradedb.term_set_bitset_max_density_unique;
RESET paradedb.term_set_bitset_max_density_multi;

SELECT
    (SELECT count(*) FROM result_q3_gallop) AS gallop_count,
    (SELECT count(*) FROM result_q3_linear) AS linear_count,
    (SELECT count(*) FROM result_q3_gallop) = (SELECT count(*) FROM result_q3_linear) AS counts_match;

SELECT 'q3 diff (must be empty)' AS check, source, id
FROM (
    SELECT 'gallop-only' AS source, id FROM result_q3_gallop
    EXCEPT
    SELECT 'gallop-only', id FROM result_q3_linear
    UNION ALL
    SELECT 'linear-only' AS source, id FROM result_q3_linear
    EXCEPT
    SELECT 'linear-only', id FROM result_q3_gallop
) AS diff
ORDER BY source, id;

DROP TABLE result_q3_gallop;
DROP TABLE result_q3_linear;

-- ============================================================================
-- Cleanup
-- ============================================================================

DROP TABLE IF EXISTS ts_outer CASCADE;
DROP TABLE IF EXISTS ts_unique CASCADE;
DROP TABLE IF EXISTS ts_multi CASCADE;
DROP TABLE IF EXISTS ts_sorted CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET enable_nestloop;
RESET enable_mergejoin;
RESET paradedb.enable_join_custom_scan;

-- Regression tests for the D-aware TermSet dispatch gates and the three
-- paradedb.term_set_* density GUCs that drive them.
--
-- # Setup
--
-- Two inner-side tables expose the two D shapes the dispatcher cares about:
--   - ts_unique: N=10000, fk = id, every value distinct → D = 1.
--   - ts_multi : N=10000, fk = ((i-1) % 100) + 1     → dict_size = 100, D = 100.
-- Plus one sorted variant for gallop coverage:
--   - ts_sorted: N=10000, fk = id, sort_by = 'fk ASC'.
--
-- ts_outer (100 rows) is the hash-join build side. K is controlled per-test
-- by `ts_outer.id <= K` on the build-side WHERE clause; the predicate filters
-- ts_outer to K rows, the hash table has K entries, and those K ids get
-- pushed into the inner scan as a TermSet of size K.
--
-- # Dispatch contract
--
-- Each inner-scan dispatch decision is captured as a token in the EXPLAIN
-- ANALYZE plan:
--
--     PgSearchScan: segments=1, dynamic_filters=1, dynamic_filter_pushdown=<strategy>, ...
--
-- where <strategy> ∈ {bitset_from_postings, linear, gallop, empty, true}.
-- The captured EXPLAIN block in `expected/term_set_dispatch.out` is the
-- assertion contract; the SELECT below it captures the row contents for
-- correctness.
--
-- # Threshold defaults (matching tantivy's TermSetStrategyConfig)
--
--   bitset_max_density_unique = 1/2000 = 0.0005    (D=1 path)
--   bitset_max_density_multi  = 1/200  = 0.005     (D≥2 path)
--   gallop_max_density        = 1.0                (no-op gate)
--
-- The bitset gates use <= so the threshold value itself admits bitset.

-- Plan-stabilizing GUCs (mirror join_hash.sql).
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;
SET enable_nestloop = OFF;
SET enable_mergejoin = OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_join_custom_scan = ON;

-- ============================================================================
-- Fixtures
-- ============================================================================

DROP TABLE IF EXISTS ts_outer CASCADE;
DROP TABLE IF EXISTS ts_unique CASCADE;
DROP TABLE IF EXISTS ts_multi CASCADE;
DROP TABLE IF EXISTS ts_sorted CASCADE;

CREATE TABLE ts_outer (id INTEGER PRIMARY KEY, val TEXT);
INSERT INTO ts_outer SELECT i, 'doc' FROM generate_series(1, 100) i;
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
-- TEST A: defaults check on a representative cell
-- ============================================================================
-- First test in the file: pin one cell against the un-overridden defaults so
-- the test catches accidental default-value drift in TermSetStrategyConfig
-- or the paradedb GUC initializers.
--
-- ts_unique with K=4 → K/N = 0.0004 < 0.0005 → admit bitset.

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 4
ORDER BY ts_unique.id ASC
LIMIT 10;

SELECT ts_outer.id, ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 4
ORDER BY ts_unique.id ASC
LIMIT 10;

-- ============================================================================
-- TEST B: D=1 (unique) bitset gate, K/N just at the threshold
-- ============================================================================
-- K=5, N=10000 → K/N = 0.0005 = bitset_max_density_unique. The `<=` gate
-- admits the threshold value itself, so this routes to bitset_from_postings.

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 5
ORDER BY ts_unique.id ASC
LIMIT 10;

SELECT ts_outer.id, ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 5
ORDER BY ts_unique.id ASC
LIMIT 10;

-- ============================================================================
-- TEST C: D=1 (unique) bitset gate, K/N just above the threshold
-- ============================================================================
-- K=6, N=10000 → K/N = 0.0006 > 0.0005 → falls through to linear.

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 6
ORDER BY ts_unique.id ASC
LIMIT 10;

SELECT ts_outer.id, ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 6
ORDER BY ts_unique.id ASC
LIMIT 10;

-- ============================================================================
-- TEST D: D=100 (multi) bitset gate, K/N just at the threshold
-- ============================================================================
-- K=50, N=10000 → K/N = 0.005 = bitset_max_density_multi → bitset.
-- ts_multi has D=100 so the dispatch site reads dict_size=100, computes
-- D=100, and selects the multi gate.

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_multi.id
FROM ts_outer JOIN ts_multi ON ts_outer.id = ts_multi.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 50
ORDER BY ts_multi.id ASC
LIMIT 10;

SELECT ts_outer.id, ts_multi.id
FROM ts_outer JOIN ts_multi ON ts_outer.id = ts_multi.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 50
ORDER BY ts_multi.id ASC
LIMIT 10;

-- ============================================================================
-- TEST E: D=100 (multi) bitset gate, K/N just above the threshold
-- ============================================================================
-- K=60, N=10000 → K/N = 0.006 > 0.005 → linear.

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_multi.id
FROM ts_outer JOIN ts_multi ON ts_outer.id = ts_multi.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 60
ORDER BY ts_multi.id ASC
LIMIT 10;

SELECT ts_outer.id, ts_multi.id
FROM ts_outer JOIN ts_multi ON ts_outer.id = ts_multi.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 60
ORDER BY ts_multi.id ASC
LIMIT 10;

-- ============================================================================
-- TEST F: sorted segment admits Gallop at any density
-- ============================================================================
-- ts_sorted is sorted ASC by fk, so the gallop precondition holds. With the
-- default gallop_max_density = 1.0, even K/N = 0.01 (K=100) admits gallop.
-- (Sanity check that the relaxed default didn't break the gallop path.)

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_sorted.id
FROM ts_outer JOIN ts_sorted ON ts_outer.id = ts_sorted.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 100
ORDER BY ts_sorted.id ASC
LIMIT 10;

SELECT ts_outer.id, ts_sorted.id
FROM ts_outer JOIN ts_sorted ON ts_outer.id = ts_sorted.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 100
ORDER BY ts_sorted.id ASC
LIMIT 10;

-- ============================================================================
-- TEST G: bitset_max_density_unique GUC override → linear at low K/N
-- ============================================================================
-- Even K=4 (default-admitted) routes to linear when the gate is forced to 0.

SET paradedb.term_set_bitset_max_density_unique = 0.0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 4
ORDER BY ts_unique.id ASC
LIMIT 10;

RESET paradedb.term_set_bitset_max_density_unique;

-- ============================================================================
-- TEST H: bitset_max_density_unique GUC override → bitset at high K/N
-- ============================================================================
-- K=20 (K/N = 0.002, normally linear) is admitted when the gate is widened.

SET paradedb.term_set_bitset_max_density_unique = 1.0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_unique.id
FROM ts_outer JOIN ts_unique ON ts_outer.id = ts_unique.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 20
ORDER BY ts_unique.id ASC
LIMIT 10;

RESET paradedb.term_set_bitset_max_density_unique;

-- ============================================================================
-- TEST I: bitset_max_density_multi GUC override → linear at low K/N
-- ============================================================================
-- K=40 on ts_multi (K/N = 0.004, default-admitted at 1/200) goes to linear
-- when the multi gate is forced to 0.

SET paradedb.term_set_bitset_max_density_multi = 0.0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_multi.id
FROM ts_outer JOIN ts_multi ON ts_outer.id = ts_multi.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 40
ORDER BY ts_multi.id ASC
LIMIT 10;

RESET paradedb.term_set_bitset_max_density_multi;

-- ============================================================================
-- TEST J: bitset_max_density_multi GUC override → bitset at high K/N
-- ============================================================================
-- K=100 on ts_multi (K/N = 0.01, default-linear) is admitted when the gate
-- is widened.

SET paradedb.term_set_bitset_max_density_multi = 1.0;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_multi.id
FROM ts_outer JOIN ts_multi ON ts_outer.id = ts_multi.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 100
ORDER BY ts_multi.id ASC
LIMIT 10;

RESET paradedb.term_set_bitset_max_density_multi;

-- ============================================================================
-- TEST K: gallop_enabled = OFF on sorted segment falls through to D=1 gate
-- ============================================================================
-- Gallop is structurally admissible (ts_sorted is sort-matching) but the
-- kill switch rejects it; dispatch then falls through to the unique-D
-- bitset gate. K=4 (K/N = 0.0004) → bitset.

SET paradedb.term_set_gallop_enabled = OFF;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT ts_outer.id, ts_sorted.id
FROM ts_outer JOIN ts_sorted ON ts_outer.id = ts_sorted.fk
WHERE ts_outer.val @@@ 'doc' AND ts_outer.id <= 4
ORDER BY ts_sorted.id ASC
LIMIT 10;

RESET paradedb.term_set_gallop_enabled;

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

\i common/common_setup.sql

-- ============================================================================
-- NUMERIC AGGREGATE PUSHDOWN TESTS
-- ============================================================================
-- SUM/AVG/MIN/MAX/COUNT over NUMERIC columns route to the DataFusion backend
-- of the aggregate custom scan:
-- 1. Numeric64 (scaled I64): NUMERIC(p,s) where p <= 18
-- 2. NumericBytes (decimal-bytes): NUMERIC(p,s) where p > 18
-- Unbounded NUMERIC (no typmod) falls back to Postgres.
-- ============================================================================

SET paradedb.enable_aggregate_custom_scan TO on;
SET max_parallel_workers_per_gather = 2;

DROP TABLE IF EXISTS numagg CASCADE;
CREATE TABLE numagg (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price NUMERIC(15, 2),
    weight NUMERIC(38, 9),
    amount NUMERIC(78, 0),
    free NUMERIC
);

INSERT INTO numagg (description, category, price, weight, amount, free) VALUES
    ('apple laptop', 'electronics', 1000.10, 1.500000001, 100000000000000000000000000000000000000000000000000000000000000000000000000001, 1.10),
    ('apple phone', 'electronics', 2000.25, 2.250000002, 200000000000000000000000000000000000000000000000000000000000000000000000000002, 2.20),
    ('banana basket', 'grocery', 10.05, 0.100000003, 33, 0.30),
    ('cherry basket', 'grocery', 20.90, 0.200000004, 44, NULL),
    ('empty crate', 'grocery', NULL, NULL, NULL, NULL);

CREATE INDEX numagg_idx ON numagg USING paradedb (
    id,
    (description::pdb.unicode_words),
    (category::pdb.literal),
    price, weight, amount, free
) WITH (key_field = 'id');

-- Deterministic worker selection in fallback EXPLAINs
ANALYZE numagg;

-- ============================================================================
-- PART 1: Scalar aggregates, Numeric64 storage: NUMERIC(15,2)
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(price), AVG(price), MIN(price), MAX(price), COUNT(price) FROM numagg WHERE description ||| 'apple OR basket OR crate';
SELECT SUM(price), AVG(price), MIN(price), MAX(price), COUNT(price) FROM numagg WHERE description ||| 'apple OR basket OR crate';

-- Cross-check against Postgres without pushdown
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT SUM(price), AVG(price), MIN(price), MAX(price), COUNT(price) FROM numagg WHERE description ||| 'apple OR basket OR crate';
SET paradedb.enable_aggregate_custom_scan TO on;

-- ============================================================================
-- PART 2: Scalar aggregates, NumericBytes storage: NUMERIC(38,9)
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(weight), AVG(weight), MIN(weight), MAX(weight), COUNT(weight) FROM numagg WHERE description ||| 'apple OR basket OR crate';
SELECT SUM(weight), AVG(weight), MIN(weight), MAX(weight), COUNT(weight) FROM numagg WHERE description ||| 'apple OR basket OR crate';

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT SUM(weight), AVG(weight), MIN(weight), MAX(weight), COUNT(weight) FROM numagg WHERE description ||| 'apple OR basket OR crate';
SET paradedb.enable_aggregate_custom_scan TO on;

-- ============================================================================
-- PART 3: Scalar aggregates, NumericBytes storage: NUMERIC(78,0)
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(amount), AVG(amount), MIN(amount), MAX(amount), COUNT(amount) FROM numagg WHERE description ||| 'apple OR basket OR crate';
SELECT SUM(amount), AVG(amount), MIN(amount), MAX(amount), COUNT(amount) FROM numagg WHERE description ||| 'apple OR basket OR crate';

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT SUM(amount), AVG(amount), MIN(amount), MAX(amount), COUNT(amount) FROM numagg WHERE description ||| 'apple OR basket OR crate';
SET paradedb.enable_aggregate_custom_scan TO on;

-- ============================================================================
-- PART 4: Unbounded NUMERIC falls back to Postgres
-- ============================================================================

-- Serial for the fallback EXPLAIN so no worker-count lines appear
SET max_parallel_workers_per_gather = 0;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(free) FROM numagg WHERE description ||| 'apple OR basket OR crate';
SELECT SUM(free) FROM numagg WHERE description ||| 'apple OR basket OR crate';
SET max_parallel_workers_per_gather = 2;

-- COUNT on an unbounded NUMERIC column still pushes down
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(free) FROM numagg WHERE description ||| 'apple OR basket OR crate';
SELECT COUNT(free) FROM numagg WHERE description ||| 'apple OR basket OR crate';

-- ============================================================================
-- PART 5: GROUP BY a text column, numeric aggregates
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, SUM(price), AVG(amount), MIN(weight), MAX(price) FROM numagg WHERE description ||| 'apple OR basket OR crate' GROUP BY category ORDER BY category;
SELECT category, SUM(price), AVG(amount), MIN(weight), MAX(price) FROM numagg WHERE description ||| 'apple OR basket OR crate' GROUP BY category ORDER BY category;

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT category, SUM(price), AVG(amount), MIN(weight), MAX(price) FROM numagg WHERE description ||| 'apple OR basket OR crate' GROUP BY category ORDER BY category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- ============================================================================
-- PART 6: GROUP BY a NUMERIC column
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT price, COUNT(*) FROM numagg WHERE description ||| 'apple OR basket' GROUP BY price ORDER BY price;
SELECT price, COUNT(*) FROM numagg WHERE description ||| 'apple OR basket' GROUP BY price ORDER BY price;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT amount, COUNT(*) FROM numagg WHERE description ||| 'apple OR basket' GROUP BY amount ORDER BY amount;
SELECT amount, COUNT(*) FROM numagg WHERE description ||| 'apple OR basket' GROUP BY amount ORDER BY amount;

-- ============================================================================
-- PART 6b: ORDER BY a numeric aggregate with LIMIT (TopK)
-- ============================================================================

-- SUM sorts natively: decimal-bytes ordering matches numeric ordering
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, SUM(amount) s FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category ORDER BY s DESC LIMIT 1;
SELECT category, SUM(amount) s FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category ORDER BY s DESC LIMIT 1;

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT category, SUM(amount) s FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category ORDER BY s DESC LIMIT 1;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Numeric64 SUM sorts the same way
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, SUM(price) s FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category ORDER BY s ASC LIMIT 1;
SELECT category, SUM(price) s FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category ORDER BY s ASC LIMIT 1;

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT category, SUM(price) s FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category ORDER BY s ASC LIMIT 1;
SET paradedb.enable_aggregate_custom_scan TO on;

-- AVG blobs do not sort; TopK is skipped and Postgres sorts above the scan
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, AVG(price) a FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category ORDER BY a ASC LIMIT 1;
SELECT category, AVG(price) a FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category ORDER BY a ASC LIMIT 1;

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT category, AVG(price) a FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category ORDER BY a ASC LIMIT 1;
SET paradedb.enable_aggregate_custom_scan TO on;

-- ============================================================================
-- PART 7: NaN handling
-- ============================================================================

INSERT INTO numagg (description, category, price, weight, amount) VALUES
    ('durian crate', 'grocery', 'NaN', 'NaN', 'NaN');
ANALYZE numagg;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(price), MIN(price), MAX(price) FROM numagg WHERE description ||| 'durian OR basket';
SELECT SUM(price), MIN(price), MAX(price) FROM numagg WHERE description ||| 'durian OR basket';

SELECT SUM(weight), MIN(weight), MAX(weight) FROM numagg WHERE description ||| 'durian OR basket';
SELECT SUM(amount), AVG(amount), MIN(amount), MAX(amount) FROM numagg WHERE description ||| 'durian OR basket';

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT SUM(price), MIN(price), MAX(price) FROM numagg WHERE description ||| 'durian OR basket';
SELECT SUM(weight), MIN(weight), MAX(weight) FROM numagg WHERE description ||| 'durian OR basket';
SELECT SUM(amount), AVG(amount), MIN(amount), MAX(amount) FROM numagg WHERE description ||| 'durian OR basket';
SET paradedb.enable_aggregate_custom_scan TO on;

-- TopK over a group whose SUM is NaN: NaN sorts highest, matching Postgres
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, SUM(price) s FROM numagg WHERE description ||| 'durian OR basket OR apple' GROUP BY category ORDER BY s DESC LIMIT 1;
SELECT category, SUM(price) s FROM numagg WHERE description ||| 'durian OR basket OR apple' GROUP BY category ORDER BY s DESC LIMIT 1;

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT category, SUM(price) s FROM numagg WHERE description ||| 'durian OR basket OR apple' GROUP BY category ORDER BY s DESC LIMIT 1;
SET paradedb.enable_aggregate_custom_scan TO on;

-- ============================================================================
-- PART 8: Empty result set and FILTER clause
-- ============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(amount), AVG(amount), COUNT(*) FROM numagg WHERE description ||| 'nonexistent';
SELECT SUM(amount), AVG(amount), COUNT(*) FROM numagg WHERE description ||| 'nonexistent';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(price) FILTER (WHERE category = 'grocery') FROM numagg WHERE description ||| 'apple OR basket';
SELECT SUM(price) FILTER (WHERE category = 'grocery') FROM numagg WHERE description ||| 'apple OR basket';

-- ============================================================================
-- PART 9: Declined shapes fall back to Postgres
-- ============================================================================

-- Serial for the fallback EXPLAINs so no worker-count lines appear
SET max_parallel_workers_per_gather = 0;

-- HAVING over a numeric aggregate
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, SUM(price) FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category HAVING SUM(price) > 100 ORDER BY category;
SELECT category, SUM(price) FROM numagg WHERE description ||| 'apple OR basket' GROUP BY category HAVING SUM(price) > 100 ORDER BY category;

-- SUM(DISTINCT numeric)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(DISTINCT price) FROM numagg WHERE description ||| 'apple OR basket';
SELECT SUM(DISTINCT price) FROM numagg WHERE description ||| 'apple OR basket';

-- stddev is not supported on NUMERIC columns
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT STDDEV(price) FROM numagg WHERE description ||| 'apple OR basket';
SELECT STDDEV(price) FROM numagg WHERE description ||| 'apple OR basket';

-- ============================================================================
-- PART 10: Aggregate over a join
-- ============================================================================

SET max_parallel_workers_per_gather = 2;
SET paradedb.enable_join_custom_scan TO on;

DROP TABLE IF EXISTS numagg_lines CASCADE;
CREATE TABLE numagg_lines (
    id SERIAL PRIMARY KEY,
    numagg_id INTEGER,
    note TEXT
);

INSERT INTO numagg_lines (numagg_id, note) VALUES
    (1, 'first line'),
    (1, 'second line'),
    (2, 'third line'),
    (3, 'fourth line');

CREATE INDEX numagg_lines_idx ON numagg_lines USING paradedb (
    id, numagg_id, (note::pdb.unicode_words)
) WITH (key_field = 'id');
ANALYZE numagg_lines;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(n.price), SUM(n.amount), MIN(n.weight) FROM numagg n JOIN numagg_lines l ON l.numagg_id = n.id WHERE n.description ||| 'apple OR basket';
SELECT SUM(n.price), SUM(n.amount), MIN(n.weight) FROM numagg n JOIN numagg_lines l ON l.numagg_id = n.id WHERE n.description ||| 'apple OR basket';

SET paradedb.enable_aggregate_custom_scan TO off;
SELECT SUM(n.price), SUM(n.amount), MIN(n.weight) FROM numagg n JOIN numagg_lines l ON l.numagg_id = n.id WHERE n.description ||| 'apple OR basket';
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE numagg_lines CASCADE;
DROP TABLE numagg CASCADE;

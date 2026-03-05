-- Test for the JoinScan Custom Scan planning
-- This test verifies that the join custom scan is proposed when:
-- 1. Query has a LIMIT clause
-- 2. At least one side has a BM25 index with a @@@ predicate

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;

-- Create test tables
-- Using explicit IDs in distinct ranges to differentiate from ctids:
-- Suppliers: IDs 151-154
-- Products: IDs 201-208
CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER,
    price DECIMAL(10,2)
);

CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    contact_info TEXT,
    country TEXT
);

-- Insert test data with explicit IDs
INSERT INTO suppliers (id, name, contact_info, country) VALUES
(151, 'TechCorp', 'contact@techcorp.com wireless technology', 'USA'),
(152, 'GlobalSupply', 'info@globalsupply.com international shipping', 'UK'),
(153, 'FastParts', 'sales@fastparts.com quick delivery', 'Germany'),
(154, 'QualityFirst', 'quality@first.com premium products', 'Japan');

INSERT INTO products (id, name, description, supplier_id, price) VALUES
(201, 'Wireless Mouse', 'Ergonomic wireless mouse with Bluetooth connectivity', 151, 29.99),
(202, 'USB Cable', 'High-speed USB-C cable for fast data transfer', 152, 9.99),
(203, 'Keyboard', 'Mechanical keyboard with RGB lighting', 151, 89.99),
(204, 'Monitor Stand', 'Adjustable monitor stand for ergonomic setup', 153, 49.99),
(205, 'Webcam', 'HD webcam for video conferencing', 154, 59.99),
(206, 'Headphones', 'Wireless noise-canceling headphones with premium sound', 151, 199.99),
(207, 'Mouse Pad', 'Large gaming mouse pad with wireless charging', 152, 39.69),
(208, 'Cable Organizer', 'Desktop cable organizer for clean setup', 153, 14.99);

-- Create BM25 indexes on both tables
-- Note: JoinScan requires all join key columns and ORDER BY columns to be fast fields
CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, description, supplier_id, price)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}, "price": {"fast": true}}');
CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name, contact_info, country)
WITH (key_field = 'id');

-- Make sure the GUC is enabled
SET paradedb.enable_join_custom_scan = on;



-- =============================================================================
-- TEST 36: Join on sorted keys (Both sides sorted on join key)
-- =============================================================================

DROP TABLE IF EXISTS sorted_t1 CASCADE;
DROP TABLE IF EXISTS sorted_t2 CASCADE;

CREATE TABLE sorted_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE sorted_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

INSERT INTO sorted_t1 SELECT i, 'val ' || i FROM generate_series(1, 1000) i;
INSERT INTO sorted_t2 SELECT i, (i % 1000) + 1, 'val ' || i FROM generate_series(1, 1000) i;

-- Indexes sorted by join key
-- t1 sorted by id
CREATE INDEX sorted_t1_idx ON sorted_t1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}');

-- t2 sorted by t1_id (the foreign key)
CREATE INDEX sorted_t2_idx ON sorted_t2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}');

ANALYZE sorted_t1;
ANALYZE sorted_t2;

-- Join on t1.id = t2.t1_id
-- Both are sorted on the join key (ASC NULLS FIRST)
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val
FROM sorted_t1 t1
JOIN sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

SELECT t1.val, t2.val
FROM sorted_t1 t1
JOIN sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

-- =============================================================================
-- TEST 36b: OFFSET + LIMIT on sorted join keys
-- PostgreSQL's limit_tuples includes the offset (5+10=15), so JoinScan passes
-- fetch=15 to DataFusion. The EXPLAIN should show SortExec: TopK(fetch=15)
-- wrapping StripOrderingExec. PostgreSQL's outer Limit applies the offset.
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val
FROM sorted_t1 t1
JOIN sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
OFFSET 5 LIMIT 10;

SELECT t1.val, t2.val
FROM sorted_t1 t1
JOIN sorted_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
OFFSET 5 LIMIT 10;

-- =============================================================================
-- TEST 37: Multi-segment sorted join
-- =============================================================================

DROP TABLE IF EXISTS multi_seg_1 CASCADE;
DROP TABLE IF EXISTS multi_seg_2 CASCADE;

CREATE TABLE multi_seg_1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE multi_seg_2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

-- Force multiple segments using small mutable_segment_rows
CREATE INDEX multi_seg_1_idx ON multi_seg_1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}', mutable_segment_rows = 10);

CREATE INDEX multi_seg_2_idx ON multi_seg_2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}', mutable_segment_rows = 10);

-- Insert 100 rows, should create ~10 segments each
INSERT INTO multi_seg_1 SELECT i, 'val ' || i FROM generate_series(1, 100) i;
INSERT INTO multi_seg_2 SELECT i, (i % 100) + 1, 'val ' || i FROM generate_series(1, 100) i;

ANALYZE multi_seg_1;
ANALYZE multi_seg_2;

-- Verify SortMergeJoin is used with multi-segment indexes
-- MultiSegmentPlan exposes N partitions. SortMergeJoin should work.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val
FROM multi_seg_1 t1
JOIN multi_seg_2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

SELECT t1.val, t2.val
FROM multi_seg_1 t1
JOIN multi_seg_2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

-- =============================================================================
-- TEST 38: Recursive SortMergeJoin (3 tables sorted by t1.id)
-- =============================================================================

DROP TABLE IF EXISTS recursive_smj_1 CASCADE;
DROP TABLE IF EXISTS recursive_smj_2 CASCADE;
DROP TABLE IF EXISTS recursive_smj_3 CASCADE;

CREATE TABLE recursive_smj_1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE recursive_smj_2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);
CREATE TABLE recursive_smj_3 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

INSERT INTO recursive_smj_1 SELECT i, 'val ' || i FROM generate_series(1, 100) i;
INSERT INTO recursive_smj_2 SELECT i, i, 'val ' || i FROM generate_series(1, 100) i;
INSERT INTO recursive_smj_3 SELECT i, i, 'val ' || i FROM generate_series(1, 100) i;

-- Index for t1 sorted by id
CREATE INDEX recursive_smj_1_idx ON recursive_smj_1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}');

-- Index for t2 sorted by t1_id
CREATE INDEX recursive_smj_2_idx ON recursive_smj_2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}');

-- Index for t3 sorted by t1_id
CREATE INDEX recursive_smj_3_idx ON recursive_smj_3 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}');

ANALYZE recursive_smj_1;
ANALYZE recursive_smj_2;
ANALYZE recursive_smj_3;

-- Join 3 tables on t1.id
-- t1.id = t2.t1_id AND t1.id = t3.t1_id
-- All indexes are sorted by the respective join keys.
-- Should result in recursive SortMergeJoins without any SortExecs.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val, t3.val
FROM recursive_smj_1 t1
JOIN recursive_smj_2 t2 ON t1.id = t2.t1_id
JOIN recursive_smj_3 t3 ON t1.id = t3.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

SELECT t1.val, t2.val, t3.val
FROM recursive_smj_1 t1
JOIN recursive_smj_2 t2 ON t1.id = t2.t1_id
JOIN recursive_smj_3 t3 ON t1.id = t3.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.id ASC NULLS FIRST
LIMIT 10;

-- =============================================================================
-- TEST 39: TopK dynamic filter pushdown through SortMergeJoin
-- ORDER BY differs from join key => SortExec(TopK) stays in the plan.
-- Multiple segments ensure the scan produces multiple batches so TopK can
-- tighten its threshold between batches and the pre-filter actually prunes.
-- =============================================================================

DROP TABLE IF EXISTS dyn_filter_t1 CASCADE;
DROP TABLE IF EXISTS dyn_filter_t2 CASCADE;

CREATE TABLE dyn_filter_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE dyn_filter_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

-- Create indexes BEFORE inserting data so inserts go through the mutable
-- segment pathway, producing multiple segments (index-build on existing data
-- merges everything into one segment).
CREATE INDEX dyn_filter_t1_idx ON dyn_filter_t1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}', mutable_segment_rows = 10000);

CREATE INDEX dyn_filter_t2_idx ON dyn_filter_t2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}', mutable_segment_rows = 10000);

INSERT INTO dyn_filter_t1 SELECT i, 'val ' || i FROM generate_series(1, 20000) i;
INSERT INTO dyn_filter_t2 SELECT i, (i % 20000) + 1, 'val ' || i FROM generate_series(1, 20000) i;

ANALYZE dyn_filter_t1;
ANALYZE dyn_filter_t2;

-- EXPLAIN: check that dynamic_filters appear on the scan
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.val, t2.val
FROM dyn_filter_t1 t1
JOIN dyn_filter_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.val ASC
LIMIT 10;

-- Cap the scanner batch size so TopK can tighten its threshold between batches.
SET paradedb.dynamic_filter_batch_size = 8192;

-- EXPLAIN ANALYZE: rows_pruned should be > 0 with multiple segments
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.val, t2.val
FROM dyn_filter_t1 t1
JOIN dyn_filter_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.val ASC
LIMIT 10;

-- Verify results
SELECT t1.val, t2.val
FROM dyn_filter_t1 t1
JOIN dyn_filter_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val'
ORDER BY t1.val ASC
LIMIT 10;

-- =============================================================================
-- TEST 39b: TopK dynamic filter does not prune NULLs
-- TopK emits "col IS NULL OR col < threshold". Rows with NULL in the ORDER BY
-- column must survive the pre-filter (nulls_pass=true) and be returned when
-- they belong in the top-K. Without nulls_pass, the pre-filter would
-- incorrectly discard NULLs.
--
-- Uses DESC NULLS FIRST so NULLs sort first and belong in the top-K result.
-- NULLs are placed at high IDs so they land in a later scan batch (after TopK
-- has already tightened its threshold from earlier batches). This ensures the
-- pre-filter is active when it encounters NULL values.
-- =============================================================================

DROP TABLE IF EXISTS null_val_t1 CASCADE;
DROP TABLE IF EXISTS null_val_t2 CASCADE;

CREATE TABLE null_val_t1 (id INTEGER PRIMARY KEY, val TEXT);
CREATE TABLE null_val_t2 (id INTEGER PRIMARY KEY, t1_id INTEGER, val TEXT);

CREATE INDEX null_val_t1_idx ON null_val_t1 USING bm25 (id, val)
WITH (key_field = 'id', sort_by = 'id ASC NULLS FIRST', text_fields = '{"val": {"fast": true}}', mutable_segment_rows = 10000);

CREATE INDEX null_val_t2_idx ON null_val_t2 USING bm25 (id, t1_id, val)
WITH (key_field = 'id', sort_by = 't1_id ASC NULLS FIRST', numeric_fields = '{"t1_id": {"fast": true}}', mutable_segment_rows = 10000);

-- 20K rows. Most have non-NULL val, but the last 10 (ids 19991-20000) are NULL.
-- With mutable_segment_rows=10000 the NULLs land in segment 2's later batch,
-- which is processed after TopK has updated its threshold.
INSERT INTO null_val_t1
  SELECT i,
         CASE WHEN i > 19990 THEN NULL ELSE 'val ' || i END
  FROM generate_series(1, 20000) i;
INSERT INTO null_val_t2
  SELECT i, (i % 20000) + 1, 'val ' || i
  FROM generate_series(1, 20000) i;

ANALYZE null_val_t1;
ANALYZE null_val_t2;

-- DESC NULLS FIRST: NULLs belong in the top 25.
-- The IS NULL OR pattern is decomposed into a PreFilter with nulls_pass=true.
-- EXPLAIN ANALYZE shows rows_pruned > 0 proving the pre-filter is active
-- (without the IS NULL OR decomposition, rows_pruned would be 0).
-- The NULLs in the result prove they survived the pre-filter correctly.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.val DESC NULLS FIRST, t1.id
LIMIT 25;

EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF)
SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.val DESC NULLS FIRST, t1.id
LIMIT 25;

SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.val DESC NULLS FIRST, t1.id
LIMIT 25;

-- =============================================================================
-- TEST 40: Explicit NULL handling with deferred columns
-- =============================================================================

-- TEST 40A: ORDER BY val ASC NULLS LAST
-- NULLs should appear last, so the top 10 should be strictly non-NULL values.
-- This verifies the dictionary decoder correctly sorts NULL_TERM_ORDINAL to the end.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.val ASC NULLS LAST
LIMIT 10;

SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.val ASC NULLS LAST
LIMIT 10;

-- TEST 40B: WHERE val IS NULL alone (no BM25 predicate)
-- Should fetch exactly the 10 NULL rows. 
-- Verifies the scanner can yield rows when the only filter is a NULL check.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val IS NULL
ORDER BY t1.id
LIMIT 25;

SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val IS NULL
ORDER BY t1.id
LIMIT 25;

-- TEST 40C: Mixed NULL and non-NULL rows in LIMIT results
-- ORDER BY id DESC limits to the boundary where NULLs and non-NULLs meet.
-- IDs 19991-20000 are NULL, IDs <= 19990 are non-NULL.
-- A LIMIT 15 should return exactly 10 NULLs and 5 non-NULLs mixed in the same output batch.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.id DESC
LIMIT 15;

SELECT t1.id, t1.val
FROM null_val_t1 t1
JOIN null_val_t2 t2 ON t1.id = t2.t1_id
WHERE t1.val @@@ 'val' OR t1.val IS NULL
ORDER BY t1.id DESC
LIMIT 15;
-- =============================================================================
-- CLEANUP
-- =============================================================================

RESET paradedb.dynamic_filter_batch_size;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS level1 CASCADE;
DROP TABLE IF EXISTS level2 CASCADE;
DROP TABLE IF EXISTS level3 CASCADE;
DROP TABLE IF EXISTS level4 CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS customers CASCADE;
DROP TABLE IF EXISTS inventory CASCADE;
DROP TABLE IF EXISTS warehouses CASCADE;
DROP TABLE IF EXISTS items CASCADE;
DROP TABLE IF EXISTS item_types CASCADE;
DROP TABLE IF EXISTS large_orders CASCADE;
DROP TABLE IF EXISTS large_suppliers CASCADE;
DROP TABLE IF EXISTS docs CASCADE;
DROP TABLE IF EXISTS authors CASCADE;
DROP TABLE IF EXISTS items_with_nulls CASCADE;
DROP TABLE IF EXISTS categories_with_nulls CASCADE;
DROP TABLE IF EXISTS colors CASCADE;
DROP TABLE IF EXISTS sizes CASCADE;
DROP TABLE IF EXISTS order_items CASCADE;
DROP TABLE IF EXISTS order_details CASCADE;
DROP TABLE IF EXISTS mem_test_products CASCADE;
DROP TABLE IF EXISTS mem_test_suppliers CASCADE;
DROP TABLE IF EXISTS uuid_orders CASCADE;
DROP TABLE IF EXISTS uuid_customers CASCADE;
DROP TABLE IF EXISTS numeric_transactions CASCADE;
DROP TABLE IF EXISTS numeric_accounts CASCADE;
DROP TABLE IF EXISTS large_items CASCADE;
DROP TABLE IF EXISTS large_categories CASCADE;
DROP TABLE IF EXISTS update_test_items CASCADE;
DROP TABLE IF EXISTS update_test_refs CASCADE;
DROP TABLE IF EXISTS qgen_products CASCADE;
DROP TABLE IF EXISTS qgen_users CASCADE;
DROP TABLE IF EXISTS tiny_products CASCADE;
DROP TABLE IF EXISTS tiny_refs CASCADE;
DROP TABLE IF EXISTS hint_test_products CASCADE;
DROP TABLE IF EXISTS hint_test_categories CASCADE;
DROP TABLE IF EXISTS sorted_t1 CASCADE;
DROP TABLE IF EXISTS sorted_t2 CASCADE;
DROP TABLE IF EXISTS dyn_filter_t1 CASCADE;
DROP TABLE IF EXISTS dyn_filter_t2 CASCADE;
DROP TABLE IF EXISTS null_val_t1 CASCADE;
DROP TABLE IF EXISTS null_val_t2 CASCADE;
DROP TABLE IF EXISTS multi_seg_1 CASCADE;
DROP TABLE IF EXISTS multi_seg_2 CASCADE;
DROP TABLE IF EXISTS recursive_smj_1 CASCADE;
DROP TABLE IF EXISTS recursive_smj_2 CASCADE;
DROP TABLE IF EXISTS recursive_smj_3 CASCADE;


RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;

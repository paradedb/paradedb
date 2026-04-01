\i common/common_setup.sql

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Create test table
CALL paradedb.create_bm25_test_table(
    schema_name => 'public',
    table_name => 'mock_items'
);

CREATE INDEX idx_mock_items ON mock_items
    USING bm25 (id, description, category, rating, in_stock)
    WITH (
        key_field='id',
        text_fields='{"description": {}, "category": {"fast": true}}',
        numeric_fields='{"rating": {"fast": true}}',
        boolean_fields='{"in_stock": {"fast": true}}'
    );

-- Use a broad query to match items across multiple categories
-- ================================================================
-- Test 1: ORDER BY COUNT(*) DESC LIMIT (TopK by doc count)
-- ================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC
LIMIT 3;

SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC
LIMIT 3;

-- ================================================================
-- Test 2: ORDER BY SUM(field) DESC LIMIT (TopK by sub-aggregation)
-- ================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, SUM(rating)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY SUM(rating) DESC
LIMIT 3;

SELECT category, SUM(rating)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY SUM(rating) DESC
LIMIT 3;

-- ================================================================
-- Test 3: ORDER BY COUNT(*) ASC LIMIT (bottom K)
-- ================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) ASC
LIMIT 2;

SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) ASC
LIMIT 2;

-- ================================================================
-- Test 4: ORDER BY COUNT(*) DESC with OFFSET
-- ================================================================
SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC
LIMIT 2 OFFSET 1;

-- ================================================================
-- Test 5: Parity check — DataFusion vs Postgres native
-- ================================================================
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT category, COUNT(*), SUM(rating)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT category, COUNT(*), SUM(rating)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC;

-- ================================================================
-- Test 6: Multiple aggregates with ORDER BY one of them
-- ================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*), SUM(rating), MIN(rating), MAX(rating)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY SUM(rating) DESC
LIMIT 3;

SELECT category, COUNT(*), SUM(rating), MIN(rating), MAX(rating)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY SUM(rating) DESC
LIMIT 3;

-- ================================================================
-- Test 7: LIMIT 1 (smallest possible K)
-- ================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC
LIMIT 1;

SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC
LIMIT 1;

-- ================================================================
-- Test 8: LIMIT larger than number of groups (returns all groups)
-- ================================================================
SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC
LIMIT 100;

-- ================================================================
-- Test 9: TopK top-3 vs native top-3 (tie-breaking may differ)
-- When multiple categories share the same count, Tantivy's
-- per-segment TopK approximation may return different groups
-- than Postgres's native sort. This is expected behavior.
-- ================================================================
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC
LIMIT 3;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT category, COUNT(*)
FROM mock_items
WHERE mock_items @@@ paradedb.all()
GROUP BY category
ORDER BY COUNT(*) DESC
LIMIT 3;

DROP TABLE mock_items;

-- ================================================================
-- Test 10: NULL aggregate values — known limitation
-- Tantivy treats NULL aggregates as 0/missing, while Postgres's
-- SUM(NULL, NULL) = NULL with NULLS FIRST for DESC. This is a
-- pre-existing aggregate scan limitation, not specific to TopK.
-- TopK ordering pushdown is restricted to COUNT-only to avoid
-- compounding this issue.
-- ================================================================
CREATE TABLE agg_null_topk_test (
    id SERIAL PRIMARY KEY,
    category TEXT,
    score INT
);

CREATE INDEX idx_agg_null_topk_test ON agg_null_topk_test
USING bm25 (id, category, score)
WITH (
    key_field='id',
    text_fields='{"category": {"fast": true}}',
    numeric_fields='{"score": {"fast": true}}'
);

INSERT INTO agg_null_topk_test (category, score) VALUES
    ('null_grp', NULL),
    ('null_grp', NULL),
    ('ten_grp', 10),
    ('nine_grp', 9);

-- Native Postgres: null_grp (SUM=NULL) comes first with NULLS FIRST
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT category, SUM(score) AS s
FROM agg_null_topk_test
WHERE agg_null_topk_test @@@ paradedb.all()
GROUP BY category
ORDER BY SUM(score) DESC
LIMIT 2;

-- Custom scan: Tantivy computes SUM(NULL,NULL)=0, not NULL, so
-- null_grp doesn't sort first. This is a known Tantivy limitation.
SET paradedb.enable_aggregate_custom_scan TO on;
SELECT category, SUM(score) AS s
FROM agg_null_topk_test
WHERE agg_null_topk_test @@@ paradedb.all()
GROUP BY category
ORDER BY SUM(score) DESC
LIMIT 2;

DROP TABLE agg_null_topk_test;

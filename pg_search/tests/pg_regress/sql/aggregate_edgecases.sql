-- Test aggregate edge cases
-- 1. Large aggregation result that should error
-- 2. Aggregation after deletion to test consistency

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.global_mutable_segment_rows = 0;

-- =====================================================================
-- SECTION 1: Large Aggregation Error
-- =====================================================================

-- Test that a window aggregation that returns a very large result errors gracefully.
-- We expect an error message about the result being too large.

CREATE TABLE large_agg_test (
    id SERIAL PRIMARY KEY,
    data TEXT
);

CREATE INDEX large_agg_test_idx ON large_agg_test
USING bm25 (id, data)
WITH (
    key_field = 'id',
    text_fields = '{"data": {"fast": true}}'
);

-- Insert enough data to make the terms aggregation result > 1MB
INSERT INTO large_agg_test (data) SELECT md5(g::text) FROM generate_series(1, 50000) g;

-- Test as window function
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "data", "size": 50000}}'::jsonb) OVER ()
FROM large_agg_test
WHERE id @@@ paradedb.all()
ORDER BY id
LIMIT 1;

SELECT pdb.agg('{"terms": {"field": "data", "size": 50000}}'::jsonb) OVER ()
FROM large_agg_test
WHERE id @@@ paradedb.all()
ORDER BY id
LIMIT 1;

DROP TABLE large_agg_test;

-- =====================================================================
-- SECTION 2: Aggregation After Deletion
-- =====================================================================

-- Test that aggregations are correct after rows are deleted.

CREATE TABLE delete_agg_test (
    id INT PRIMARY KEY,
    name TEXT
);

CREATE INDEX delete_agg_test_idx ON delete_agg_test
USING bm25 (id, name)
WITH (
    key_field = 'id',
    text_fields = '{"name": {}}'
);

INSERT INTO delete_agg_test VALUES (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd'), (5, 'e');

-- Delete all but one row
DELETE FROM delete_agg_test WHERE id > 1;

-- Test as aggregate function
-- Should return 1
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM delete_agg_test WHERE id @@@ paradedb.all();
SELECT COUNT(*) FROM delete_agg_test WHERE id @@@ paradedb.all();

-- Should return {"value": 1.0}
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb) FROM delete_agg_test WHERE id @@@ paradedb.all();
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb) FROM delete_agg_test WHERE id @@@ paradedb.all();

-- Should return count: 1
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"stats": {"field": "id"}}'::jsonb) FROM delete_agg_test WHERE id @@@ paradedb.all();
SELECT pdb.agg('{"stats": {"field": "id"}}'::jsonb) FROM delete_agg_test WHERE id @@@ paradedb.all();


-- Test as window function
-- Should return 1 and {"value": 1.0}
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT
    COUNT(*) OVER (),
    pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER ()
FROM delete_agg_test
WHERE id @@@ paradedb.all()
ORDER BY id
LIMIT 1;

SELECT
    COUNT(*) OVER (),
    pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER ()
FROM delete_agg_test
WHERE id @@@ paradedb.all()
ORDER BY id
LIMIT 1;

DROP TABLE delete_agg_test;

-- =====================================================================
-- SECTION 3: MVCC Visibility Settings
-- =====================================================================

CREATE TABLE mvcc_agg_test (
    id SERIAL PRIMARY KEY,
    category TEXT
);

CREATE INDEX mvcc_agg_test_idx ON mvcc_agg_test
USING bm25 (id, category)
WITH (
    key_field = 'id',
    text_fields = '{"category": {"fast": true}}'
);

INSERT INTO mvcc_agg_test (category) VALUES ('A'), ('B'), ('A');

-- Test solve_mvcc=false in standard aggregate
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"value_count": {"field": "category"}}'::jsonb, false) FROM mvcc_agg_test WHERE id @@@ paradedb.all();
SELECT pdb.agg('{"value_count": {"field": "category"}}'::jsonb, false) FROM mvcc_agg_test WHERE id @@@ paradedb.all();

-- Test solve_mvcc=false with GROUP BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, pdb.agg('{"value_count": {"field": "id"}}'::jsonb, false) FROM mvcc_agg_test WHERE id @@@ paradedb.all() GROUP BY category ORDER BY category;
SELECT category, pdb.agg('{"value_count": {"field": "id"}}'::jsonb, false) FROM mvcc_agg_test WHERE id @@@ paradedb.all() GROUP BY category ORDER BY category;

-- Test conflicting MVCC settings (should error)
SELECT
    pdb.agg('{"value_count": {"field": "id"}}'::jsonb, false),
    pdb.agg('{"value_count": {"field": "category"}}'::jsonb, true)
FROM mvcc_agg_test WHERE id @@@ paradedb.all();

DROP TABLE mvcc_agg_test;

-- =====================================================================
-- SECTION 4: pdb.agg() with ||| operator (GitHub Issue #4456)
-- =====================================================================
-- Tests that pdb.agg() works with the ||| (text-contains) operator,
-- not just @@@. Previously, ||| caused AggregateScan to be rejected
-- because the operator was not recognized in the uses_our_operator check.

CREATE TABLE triple_pipe_agg_test (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT
);

CREATE INDEX triple_pipe_agg_test_idx ON triple_pipe_agg_test
USING bm25 (id, description, category)
WITH (key_field = 'id', text_fields = '{"category": {"fast": true}}');

INSERT INTO triple_pipe_agg_test (description, category) VALUES
    ('running shoes for men', 'footwear'),
    ('running shoes for women', 'footwear'),
    ('casual walking shoes', 'footwear'),
    ('running shorts', 'apparel'),
    ('running jacket', 'apparel');

-- Test 1: ||| operator with solve_mvcc=false — the exact failing query from issue #4456
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, false)
FROM triple_pipe_agg_test
WHERE description ||| 'running shoes';

-- This SELECT is the actual proof the bug is fixed: it previously errored with
-- "pdb.agg() must be handled by ParadeDB's custom scan..."
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, false)
FROM triple_pipe_agg_test
WHERE description ||| 'running shoes';

-- Test 2: ||| operator with solve_mvcc=true (explicit)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, true)
FROM triple_pipe_agg_test
WHERE description ||| 'running shoes';

SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, true)
FROM triple_pipe_agg_test
WHERE description ||| 'running shoes';

-- Test 3: ||| operator with solve_mvcc=false + GROUP BY
SELECT category, pdb.agg('{"value_count": {"field": "id"}}'::jsonb, false)
FROM triple_pipe_agg_test
WHERE description ||| 'running'
GROUP BY category
ORDER BY category;

-- Test 4: ||| operator with deletion + solve_mvcc=false (verify count correctness)
DELETE FROM triple_pipe_agg_test WHERE id = 1;

SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, false)
FROM triple_pipe_agg_test
WHERE description ||| 'running shoes';

-- With solve_mvcc=true, should reflect the deletion
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, true)
FROM triple_pipe_agg_test
WHERE description ||| 'running shoes';

-- Test 5: ||| with default (single-arg) pdb.agg — should also work
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb)
FROM triple_pipe_agg_test
WHERE description ||| 'running shoes';

DROP TABLE triple_pipe_agg_test;


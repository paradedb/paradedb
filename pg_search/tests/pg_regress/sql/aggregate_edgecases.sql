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

-- Test scalar subquery pattern at larger scale
-- This tests the paging-string-max benchmark pattern with more realistic data volume

-- Cleanup
DROP TABLE IF EXISTS test_pages_scale CASCADE;
DROP TABLE IF EXISTS test_metadata_scale CASCADE;

-- Create test tables
CREATE TABLE test_pages_scale (
    id TEXT PRIMARY KEY,
    content TEXT
);

CREATE TABLE test_metadata_scale (
    name TEXT PRIMARY KEY,
    value TEXT
);

-- Create BM25 index on pages
CREATE INDEX test_pages_scale_idx ON test_pages_scale
USING bm25 (id, content)
WITH (key_field = 'id');

-- Insert larger dataset (10,000 rows to simulate scale)
-- This generates page IDs like 'page-0000001' through 'page-0010000'
INSERT INTO test_pages_scale (id, content)
SELECT 
    'page-' || LPAD(i::TEXT, 7, '0'),
    'content for page ' || i
FROM generate_series(1, 10000) i;

INSERT INTO test_metadata_scale (name, value) VALUES
    ('pages-row-id-max', 'page-0005000');

-- Test 1: Scalar subquery with large dataset (benchmark pattern)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT * FROM test_pages_scale 
WHERE id @@@ paradedb.all() 
  AND id >= (SELECT value FROM test_metadata_scale WHERE name = 'pages-row-id-max')
ORDER BY id 
LIMIT 100;

SELECT * FROM test_pages_scale 
WHERE id @@@ paradedb.all() 
  AND id >= (SELECT value FROM test_metadata_scale WHERE name = 'pages-row-id-max')
ORDER BY id 
LIMIT 100;

-- Test 2: Verify result count
SELECT COUNT(*) FROM test_pages_scale 
WHERE id @@@ paradedb.all() 
  AND id >= (SELECT value FROM test_metadata_scale WHERE name = 'pages-row-id-max');

-- Test 3: Test with DESC ordering (reverse scan)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT * FROM test_pages_scale 
WHERE id @@@ paradedb.all() 
  AND id >= (SELECT value FROM test_metadata_scale WHERE name = 'pages-row-id-max')
ORDER BY id DESC 
LIMIT 100;

SELECT * FROM test_pages_scale 
WHERE id @@@ paradedb.all() 
  AND id >= (SELECT value FROM test_metadata_scale WHERE name = 'pages-row-id-max')
ORDER BY id DESC 
LIMIT 100;

-- Cleanup
DROP TABLE test_pages_scale CASCADE;
DROP TABLE test_metadata_scale CASCADE;


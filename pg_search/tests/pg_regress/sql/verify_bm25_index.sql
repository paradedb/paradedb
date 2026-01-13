-- Test paradedb.verify_bm25_index function
-- This tests the amcheck-style index verification for BM25 indexes

-- Setup: Create a test table and index
DROP TABLE IF EXISTS verify_test CASCADE;
CREATE TABLE verify_test (
    id SERIAL PRIMARY KEY,
    content TEXT,
    category TEXT,
    score INT
);

CREATE INDEX verify_test_idx ON verify_test USING bm25 (id, content, category, score) WITH (key_field = 'id');

-- Insert some test data
INSERT INTO verify_test (content, category, score) VALUES
    ('hello world', 'greeting', 10),
    ('goodbye world', 'farewell', 20),
    ('search engine', 'technology', 30),
    ('full text search', 'technology', 40),
    ('paradedb postgres', 'database', 50);

-- Test 1: Basic verification without heapallindexed option
-- Should return schema_valid, index_readable, checksums_valid, segment_metadata_valid
SELECT check_name, passed FROM paradedb.verify_bm25_index('verify_test_idx') ORDER BY check_name;

-- Test 2: Verification with heapallindexed option
-- Should also check heap references
SELECT check_name, passed FROM paradedb.verify_bm25_index('verify_test_idx', heapallindexed := true) ORDER BY check_name;

-- Test 3: Verify after more data is added
INSERT INTO verify_test (content, category, score)
SELECT 
    'test content ' || i,
    CASE WHEN i % 2 = 0 THEN 'even' ELSE 'odd' END,
    i
FROM generate_series(1, 100) i;

-- Verify the index is still valid after inserts
SELECT check_name, passed FROM paradedb.verify_bm25_index('verify_test_idx') ORDER BY check_name;

-- Test 4: Verify with heapallindexed after more data
SELECT check_name, passed FROM paradedb.verify_bm25_index('verify_test_idx', heapallindexed := true) ORDER BY check_name;

-- Test 5: Verify after some deletes
DELETE FROM verify_test WHERE id <= 3;

-- Wait for any background processes to settle
SELECT pg_sleep(0.1);

-- Run vacuum to clean up
VACUUM verify_test;

-- Verify index still valid after deletes and vacuum
SELECT check_name, passed FROM paradedb.verify_bm25_index('verify_test_idx') ORDER BY check_name;

-- Test 6: Verify with heapallindexed after deletes
SELECT check_name, passed FROM paradedb.verify_bm25_index('verify_test_idx', heapallindexed := true) ORDER BY check_name;

-- Test 7: Verify that the index can still be used for searches after verification
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, content FROM verify_test WHERE content @@@ 'test' LIMIT 5;

SELECT id, content FROM verify_test WHERE content @@@ 'test' ORDER BY id LIMIT 5;

-- Cleanup
DROP TABLE verify_test CASCADE;


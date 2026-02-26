-- Test TopN scan validation feature
-- This test validates that paradedb.check_topn_scan correctly warns when
-- queries with LIMIT cannot use TopN scan optimization

\i common/common_setup.sql

-- Create test table with appropriate data
CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'test_products'
);

-- Scenario 1: Index WITHOUT lower() but query uses lower() in ORDER BY
-- This should trigger a warning when check_topn_scan is enabled
CREATE INDEX products_base_idx ON test_products
USING bm25 (id, description, category, rating)
WITH (
    key_field='id',
    text_fields='{
        "category": {"fast": true, "tokenizer": {"type": "raw"}},
        "description": {"fast": false}
    }',
    numeric_fields='{"rating": {"fast": true}}'
);

-- Test 1: Validation OFF (default) - should not warn
\echo 'Test 1: Validation OFF (no warning expected)'
SET paradedb.check_topn_scan = false;
SELECT id, description FROM test_products
WHERE description @@@ 'shoes'
ORDER BY description  -- Missing fast field
LIMIT 5;

-- Test 2: Enable validation - should warn about missing fast field
\echo 'Test 2: Validation ON - warning expected (ORDER BY not a fast field)'
SET paradedb.check_topn_scan = true;
SELECT id, description FROM test_products
WHERE description @@@ 'shoes'
ORDER BY description  -- Not marked as fast
LIMIT 5;

-- Test 3: Proper TopN - should NOT warn
\echo 'Test 3: Valid TopN query (no warning expected)'
SELECT id, category,rating FROM test_products
WHERE category @@@ 'electronics'
ORDER BY rating DESC  -- rating is fast
LIMIT 5;

-- Test 4: Too many ORDER BY columns
DROP INDEX products_base_idx;
CREATE INDEX products_multi_idx ON test_products
USING bm25 (id, description, category, rating, created_at, last_updated_date)
WITH (
    key_field='id',
    text_fields='{
        "category": {"tokenizer": {"type": "keyword"}, "fast": true},
        "description": {"tokenizer": {"type": "keyword"}, "fast": true}
    }',
    numeric_fields='{"rating": {"fast": true}}',
    datetime_fields='{"created_at": {"fast": true}, "last_updated_date": {"fast": true}}'
);

\echo 'Test 4: Too many ORDER BY columns (warning expected)'
SELECT id FROM test_products
WHERE category @@@ 'electronics'
ORDER BY rating DESC, created_at DESC, id DESC, category DESC, description DESC, last_updated_date DESC  -- 6 columns, max is 5
LIMIT 10;

-- Test 5: Query with lower() mismatch
DROP INDEX products_multi_idx;
CREATE INDEX products_lower_idx ON test_products
USING bm25 (id, description, (lower(category)::pdb.literal), rating)
WITH (
    key_field='id'
);

\echo 'Test 5a: ORDER BY with lower() - should use TopN (no warning)'
SELECT id, category FROM test_products
WHERE category === 'Electronics'
ORDER BY lower(category) DESC  -- Matches index
LIMIT 5;

\echo 'Test 5b: ORDER BY without lower() - warning expected'
SELECT id, category FROM test_products
WHERE category === 'Electronics'
ORDER BY category DESC  -- Doesn't match - index has lower()
LIMIT 5;

-- Test 6: No LIMIT - should not validate or warn
\echo 'Test 6: No LIMIT (no validation, no warning)'
SELECT id FROM test_products
WHERE category === 'Electronics'
ORDER BY rating DESC;  -- No LIMIT, so TopN not expected

-- Test 7: EXPLAIN should show warning in logs
\echo 'Test 7: EXPLAIN with validation warning'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM test_products
WHERE category === 'Electronics'
ORDER BY category DESC  -- Should warn
LIMIT 10;

-- Test 8: Disable validation mid-session
\echo 'Test 8: Disable validation (no warning)'
SET paradedb.check_topn_scan = false;
SELECT id FROM test_products
WHERE category === 'Electronics'
ORDER BY category DESC  -- No warning now
LIMIT 10;

-- Cleanup
DROP INDEX products_lower_idx;
DROP TABLE test_products;

\i common/common_cleanup.sql

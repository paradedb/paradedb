\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

-- Test prepared statements with pdb.score() function
-- This test verifies that scores work correctly in both custom and generic plans

-- Create the BM25 index
CREATE INDEX search_idx ON mock_items
USING bm25 (id, description)
WITH (key_field='id');

-- Test 1: Basic prepared statement with score and parameter
PREPARE search_desc(text, int) AS
SELECT
  id,
  description,
  paradedb.score(id) AS score
FROM mock_items
WHERE description @@@ $1        
AND $2 = 0
ORDER BY score DESC
LIMIT 5;

-- Show plan for first execution (custom plan)
EXPLAIN (COSTS OFF)
EXECUTE search_desc('keyboard', 0);

-- Execute with custom plan (first 5 times by default)
EXECUTE search_desc('keyboard', 0);
EXECUTE search_desc('keyboard', 0);
EXECUTE search_desc('keyboard', 0);
EXECUTE search_desc('keyboard', 0);
EXECUTE search_desc('keyboard', 0);

-- Show plan for 6th execution (should be generic plan by default, but still use Custom Scan)
EXPLAIN (COSTS OFF)
EXECUTE search_desc('keyboard', 0);

-- 6th execution should use generic plan (by default)
-- This should still return scores, not NULL
EXECUTE search_desc('keyboard', 0);

DEALLOCATE search_desc;

-- Test 2: Force generic plan to ensure it works
SET plan_cache_mode = force_generic_plan;

PREPARE search_generic(text, int) AS
SELECT
  id,
  description,
  paradedb.score(id) AS score
FROM mock_items
WHERE description @@@ $1        
AND $2 = 0
ORDER BY score DESC
LIMIT 5;

-- Show plan with forced generic plan (should still use Custom Scan)
EXPLAIN (COSTS OFF)
EXECUTE search_generic('keyboard', 0);

-- This should return scores even with forced generic plan
EXECUTE search_generic('keyboard', 0);

-- Test different search terms
EXECUTE search_generic('shoes', 0);

DEALLOCATE search_generic;

-- Reset plan cache mode
RESET plan_cache_mode;

-- Test 3: Prepared statement without the parameter condition
-- This serves as a control to ensure basic functionality works
PREPARE search_simple(text) AS
SELECT
  id,
  description,
  paradedb.score(id) AS score
FROM mock_items
WHERE description @@@ $1        
ORDER BY score DESC
LIMIT 5;

SET plan_cache_mode = force_generic_plan;

-- Show plan for simple query with generic plan
EXPLAIN (COSTS OFF)
EXECUTE search_simple('keyboard');

EXECUTE search_simple('keyboard');
DEALLOCATE search_simple;

-- Test 4: Using new pdb schema
PREPARE search_pdb(text, int) AS
SELECT
  id,
  description,
  pdb.score(id) AS score
FROM mock_items
WHERE description @@@ $1        
AND $2 = 0
ORDER BY score DESC
LIMIT 5;

SET plan_cache_mode = force_generic_plan;

-- Show plan with pdb.score() function
EXPLAIN (COSTS OFF)
EXECUTE search_pdb('keyboard', 0);

EXECUTE search_pdb('keyboard', 0);
DEALLOCATE search_pdb;

-- Test 5: Verify filter conditions are actually applied in generic plans
-- This tests that $2 = 0 is properly evaluated, not just ignored
SET plan_cache_mode = force_generic_plan;

PREPARE test_filter(text, int) AS
SELECT
  id,
  description,
  pdb.score(id) AS score
FROM mock_items
WHERE description @@@ $1
AND $2 = 0
ORDER BY score DESC
LIMIT 10;

-- When $2 = 0, should return results
EXPLAIN (COSTS OFF)
EXECUTE test_filter('shoes', 0);

EXECUTE test_filter('shoes', 0);

-- When $2 = 1, should return NO results (filter condition fails)
EXPLAIN (COSTS OFF)
EXECUTE test_filter('shoes', 1);

EXECUTE test_filter('shoes', 1);

-- When $2 = 0 again, should return results
EXPLAIN (COSTS OFF)
EXECUTE test_filter('keyboard', 0);

EXECUTE test_filter('keyboard', 0);

DEALLOCATE test_filter;

-- Test 6: More complex filter with multiple parameters
PREPARE test_complex(text, int, int) AS
SELECT
  id,
  description,
  rating,
  pdb.score(id) AS score
FROM mock_items
WHERE description @@@ $1
AND rating > $2
AND $3 = 0
ORDER BY score DESC
LIMIT 10;

-- Should return shoes with rating > 3
EXPLAIN (COSTS OFF)
EXECUTE test_complex('shoes', 3, 0);

EXECUTE test_complex('shoes', 3, 0);

-- Should return NO results (filter $3 = 1 fails)
EXPLAIN (COSTS OFF)
EXECUTE test_complex('shoes', 3, 1);

EXECUTE test_complex('shoes', 3, 1);

-- Should return shoes with rating > 2
EXPLAIN (COSTS OFF)
EXECUTE test_complex('shoes', 2, 0);

EXECUTE test_complex('shoes', 2, 0);

DEALLOCATE test_complex;

DROP TABLE mock_items;

\i common/common_cleanup.sql

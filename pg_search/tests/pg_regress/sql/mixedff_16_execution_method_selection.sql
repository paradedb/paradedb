-- Test proper execution method selection for mixed fast fields
-- This test verifies that the MixedFastFieldExecState is chosen when appropriate
-- and that NormalScanExecState is not used when mixed fast fields are available

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

-- Create test table with various field types
DROP TABLE IF EXISTS exec_method_test;
CREATE TABLE exec_method_test (
    id SERIAL PRIMARY KEY,
    text_field1 TEXT,
    text_field2 TEXT,
    text_field3 TEXT,
    num_field1 INTEGER,
    num_field2 FLOAT,
    num_field3 NUMERIC,
    bool_field BOOLEAN,
    non_indexed_field TEXT
);

-- Insert test data
INSERT INTO exec_method_test (
    text_field1, text_field2, text_field3,
    num_field1, num_field2, num_field3,
    bool_field, non_indexed_field
)
SELECT
    'Text ' || i,
    'Sample ' || (i % 10),
    'Category ' || (i % 5),
    i,
    (i * 1.5)::float,
    (i * 2.25)::numeric,
    i % 2 = 0,
    'Non-indexed ' || i
FROM generate_series(1, 50) i;

-- Create index with mixed fast fields
DROP INDEX IF EXISTS exec_method_idx;
CREATE INDEX exec_method_idx ON exec_method_test
USING bm25 (
    id, text_field1, text_field2, text_field3,
    num_field1, num_field2, num_field3,
    bool_field
)
WITH (
    key_field = 'id',
    text_fields = '{"text_field1": {"tokenizer": {"type": "default"}, "fast": true}, "text_field2": {"tokenizer": {"type": "default"}, "fast": true}, "text_field3": {"tokenizer": {"type": "default"}, "fast": true}}',
    numeric_fields = '{"num_field1": {"fast": true}, "num_field2": {"fast": true}, "num_field3": {"fast": true}}',
    boolean_fields = '{"bool_field": {"fast": true}}'
);

-- Test 1: Should use MixedFastFieldExecState with multiple string fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field1, text_field2
FROM exec_method_test
WHERE text_field1 @@@ 'Text';

-- Test 2: Should use MixedFastFieldExecState with mixed string and numeric fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field1, num_field1, num_field2
FROM exec_method_test
WHERE text_field1 @@@ 'Text' AND num_field1 > 10;

-- Test 3: Should use MixedFastFieldExecState with all field types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field1, text_field2, num_field1, bool_field
FROM exec_method_test
WHERE text_field1 @@@ 'Text' AND bool_field = true;

-- Test 4: Should use StringFastFieldExecState when only one string field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field1
FROM exec_method_test
WHERE text_field1 @@@ 'Text';

-- Test 5: Should use NumericFastFieldExecState when only numeric fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT num_field1, num_field2
FROM exec_method_test
WHERE num_field1 > 25;

-- Test 6: Should NOT use any FastField method when non-indexed fields are selected
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field1, non_indexed_field
FROM exec_method_test
WHERE text_field1 @@@ 'Text';

-- Test 7: Should use MixedFastFieldExecState even with ORDER BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field1, num_field1
FROM exec_method_test
WHERE text_field1 @@@ 'Text'
ORDER BY num_field1 DESC;

-- Test 8: Should use MixedFastFieldExecState with filtering on multiple field types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text_field1, text_field2, num_field1, bool_field
FROM exec_method_test
WHERE text_field1 @@@ 'Text' 
  AND text_field2 @@@ 'Sample'
  AND num_field1 BETWEEN 10 AND 40
  AND bool_field = true;

-- Test 9: Verify correct execution method in a subquery
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT t.text_field1, t.num_field1
FROM (
    SELECT text_field1, num_field1
    FROM exec_method_test
    WHERE text_field1 @@@ 'Text' AND num_field1 > 10
) t
WHERE t.num_field1 < 30;

-- Verify actual results match expected values (not just execution method)
SELECT text_field1, text_field2, num_field1
FROM exec_method_test
WHERE text_field1 @@@ 'Text 1'
  AND num_field1 < 20
ORDER BY num_field1;

-- Clean up
DROP INDEX IF EXISTS exec_method_idx;
DROP TABLE IF EXISTS exec_method_test; 

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather;
 
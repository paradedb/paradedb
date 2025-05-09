-- Cleanup for basic mixed fast fields tests (01-04)

-- Drop the table used in these tests
DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather;
RESET enable_indexscan;

SELECT 'Basic tests cleanup complete' AS status; 

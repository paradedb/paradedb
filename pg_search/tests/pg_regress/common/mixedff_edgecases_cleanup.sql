-- Cleanup for edge cases tests (05-06, 11-12)

-- Drop the tables used in these tests
DROP TABLE IF EXISTS corner_case_test CASCADE;
DROP TABLE IF EXISTS nullable_test CASCADE;
DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather;

SELECT 'Edge cases tests cleanup complete' AS status; 

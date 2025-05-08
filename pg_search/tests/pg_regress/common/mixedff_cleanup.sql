-- Cleans up the test database

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;

\echo 'Test: Cleanup'

DROP TABLE IF EXISTS nullable_test CASCADE;
DROP TABLE IF EXISTS corner_case_test CASCADE;
DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;
DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;

SELECT 'Cleanup complete' AS status;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather; 

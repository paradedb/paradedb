-- Cleanup for advanced features tests (13-21)

-- Drop the tables used in these tests (in reverse order to handle dependencies)
DROP TABLE IF EXISTS conversion_test CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;
DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;
DROP INDEX IF EXISTS union_test_a_idx CASCADE;
DROP INDEX IF EXISTS union_test_b_idx CASCADE;
DROP TABLE IF EXISTS union_test_a CASCADE;
DROP TABLE IF EXISTS union_test_b CASCADE; 


-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather;
RESET enable_indexscan;

SELECT 'Advanced features tests cleanup complete' AS status; 

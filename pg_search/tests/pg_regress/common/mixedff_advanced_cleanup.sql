-- Cleanup for advanced features tests (13-21)

-- Drop the tables used in these tests (in reverse order to handle dependencies)
DROP TABLE IF EXISTS conversion_test CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;
DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather;

SELECT 'Advanced features tests cleanup complete' AS status; 

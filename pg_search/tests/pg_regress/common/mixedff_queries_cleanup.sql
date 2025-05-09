-- Cleanup for relational query tests (07-10)

-- Drop the tables used in these tests
DROP TABLE IF EXISTS pages CASCADE;
DROP TABLE IF EXISTS files CASCADE;
DROP TABLE IF EXISTS documents CASCADE;

-- Reset parallel workers setting to default
RESET max_parallel_workers_per_gather;

SELECT 'Relational query tests cleanup complete' AS status; 

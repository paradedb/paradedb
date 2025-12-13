-- Reproduction test case for issue #3750
-- "Parallel Index Scan fails to match rows Parallel Custom Scan does"
--
-- Issue: When query uses Parallel Custom Scan node, all expected results are returned.
-- However, when adding non-BM25 filters (like country_code = 'us'), the planner 
-- switches to Parallel Index Scan and some matching results are omitted.
--
-- Workaround: Adding `AND id @@@ pdb.all()` forces Custom Scan path.

\i common/common_setup.sql

-- Create the table structure from the issue
DROP TABLE IF EXISTS places;
CREATE TABLE places (
    id TEXT PRIMARY KEY,
    name TEXT,
    country_code TEXT
);

-- Create the BM25 index as specified in the issue
CREATE INDEX places_parade_idx ON places USING bm25 (id, name, country_code) WITH (key_field='id');

-- Insert test data that matches the issue's scenario
-- The issue mentions "assist wireless" matching rows
-- Generate enough data to trigger multiple parallel workers
INSERT INTO places (id, name, country_code)
SELECT 
    i::text,
    CASE 
        WHEN i % 10 = 0 THEN 'assist wireless'
        WHEN i % 10 = 1 THEN 'wireless assist'
        WHEN i % 10 = 2 THEN 'assist wireless service'
        WHEN i % 10 = 3 THEN 'assist wireless shop'
        WHEN i % 10 = 4 THEN 'assist wireless center'
        WHEN i % 10 = 5 THEN 'assist wireless store'
        WHEN i % 10 = 6 THEN 'assist wireless outlet'
        WHEN i % 10 = 7 THEN 'other business'
        WHEN i % 10 = 8 THEN 'random service'
        ELSE 'unrelated shop'
    END,
    CASE WHEN i % 2 = 0 THEN 'us' ELSE 'ca' END
FROM generate_series(1, 100000) i;

-- Force parallel execution settings to match user's environment
SET max_parallel_workers_per_gather = 4;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET debug_parallel_query = regress;

-- ============================================================
-- TEST 1: Query with only BM25 filter (should use Custom Scan)
-- From issue: "SELECT id, name, country_code, country_code = 'us' FROM places WHERE name &&& 'assist wireless';"
-- ============================================================
RESET paradedb.enable_custom_scan;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*) as count
FROM places 
WHERE name &&& 'assist wireless';

SELECT COUNT(*) as count
FROM places 
WHERE name &&& 'assist wireless';

-- ============================================================
-- TEST 2: Query with BM25 filter + additional filter (may use Index Scan)
-- From issue: "SELECT id, name, country_code, country_code = 'us' FROM places WHERE name &&& 'assist wireless' AND country_code = 'us';"
-- ============================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*) as count
FROM places 
WHERE name &&& 'assist wireless' AND country_code = 'us';

SELECT COUNT(*) as count
FROM places 
WHERE name &&& 'assist wireless' AND country_code = 'us';

-- ============================================================
-- TEST 3: Same queries with Index Scan forced (disable Custom Scan)
-- This is where the issue occurs according to the bug report
-- ============================================================
SET paradedb.enable_custom_scan = off;

-- EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
-- SELECT COUNT(*) as count
-- FROM places 
-- WHERE name &&& 'assist wireless';

SELECT COUNT(*) as count
FROM places 
WHERE name &&& 'assist wireless';

-- EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
-- SELECT COUNT(*) as count
-- FROM places 
-- WHERE name &&& 'assist wireless' AND country_code = 'us';

SELECT COUNT(*) as count
FROM places 
WHERE name &&& 'assist wireless' AND country_code = 'us';

-- ============================================================
-- TEST 4: Workaround mentioned in the issue
-- Adding "AND id @@@ pdb.all()" should force Custom Scan
-- ============================================================
RESET paradedb.enable_custom_scan;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*) as count
FROM places 
WHERE name &&& 'assist wireless' AND country_code = 'us' AND id @@@ paradedb.all();

SELECT COUNT(*) as count
FROM places 
WHERE name &&& 'assist wireless' AND country_code = 'us' AND id @@@ paradedb.all();

-- ============================================================
-- VERIFICATION: All three methods should return the same count
-- If there's a bug, Index Scan count will be less than others
-- ============================================================
SELECT 
    (SELECT COUNT(*) FROM places WHERE name &&& 'assist wireless' AND country_code = 'us') as custom_scan_count,
    (SELECT COUNT(*) FROM places WHERE name &&& 'assist wireless' AND country_code = 'us' AND id @@@ paradedb.all()) as workaround_count;

SET paradedb.enable_custom_scan = off;
SELECT 
    (SELECT COUNT(*) FROM places WHERE name &&& 'assist wireless' AND country_code = 'us') as index_scan_count;

-- Cleanup
RESET max_parallel_workers_per_gather;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET debug_parallel_query;
RESET paradedb.enable_custom_scan;

DROP TABLE places;

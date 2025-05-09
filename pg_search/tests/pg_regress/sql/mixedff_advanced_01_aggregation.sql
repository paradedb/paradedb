-- Tests aggregation with mixed fast fields

\i common/mixedff_advanced_setup.sql

\echo 'Test: Aggregation query'

-- Check execution plan for COUNT
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*)
FROM pages
WHERE content @@@ 'Socienty';

-- Test COUNT aggregation
SELECT COUNT(*)
FROM pages
WHERE content @@@ 'Socienty';

-- Check execution plan for other aggregations
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    COUNT(*) AS count,
    AVG(page_number) AS avg_page,
    MIN(page_number) AS min_page,
    MAX(page_number) AS max_page
FROM pages
WHERE content @@@ 'Socienty';

-- Test other aggregations
SELECT 
    COUNT(*) AS count,
    AVG(page_number) AS avg_page,
    MIN(page_number) AS min_page,
    MAX(page_number) AS max_page
FROM pages
WHERE content @@@ 'Socienty';

\i common/mixedff_advanced_cleanup.sql

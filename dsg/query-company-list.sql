-- Original Query: Filter by company list (unsorted, full result set).
-- Converted to Top-K: Ordering by `revenue_rank` (fast field) descending to find the highest-ranking contacts, limited to 10.

-- NOTES
-- 2/24 (ming): Requires semi join support https://github.com/paradedb/paradedb/pull/4226 for join custom scan pushdown
-- 2/25 (ming): At 2 workers we're 2-3X faster hot, but at 8 workers we're similar

SET work_mem = '1GB';
SET max_parallel_workers_per_gather = 8;

SET paradedb.enable_join_custom_scan TO off;
EXPLAIN ANALYZE SELECT * FROM contacts_companies_combined_full
WHERE company_id IN (
    SELECT ldf_id
    FROM company_list
    WHERE list_id IN ('2543')
)
AND company_id @@@ pdb.all()
ORDER BY revenue_rank DESC, contact_id ASC
LIMIT 10;

SET paradedb.enable_join_custom_scan TO on;
EXPLAIN ANALYZE SELECT * FROM contacts_companies_combined_full
WHERE company_id IN (
    SELECT ldf_id
    FROM company_list
    WHERE list_id IN ('2543')
)
AND company_id @@@ pdb.all()
ORDER BY revenue_rank DESC, contact_id ASC
LIMIT 10;

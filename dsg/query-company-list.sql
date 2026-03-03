-- Original Query: Filter by company list (unsorted, full result set).
-- Converted to Top-K: Ordering by `revenue_rank` (fast field) descending to find the highest-ranking contacts, limited to 10.

-- NOTES
-- 2/24 (ming): Requires semi join support https://github.com/paradedb/paradedb/pull/4226 for join custom scan pushdown
-- 2/25 (ming): We're 2-3X faster with join custom scan pushdown
-- 3/02 (stuhood): Hits 50x fewer shared buffers with the custom scan: 3611001 vs 65997

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

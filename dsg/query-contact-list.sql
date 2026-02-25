-- Original Query: Filter by contact list (unsorted, full result set).
-- Converted to Top-K: Ordering by `revenue_rank` (fast field) descending to find the highest-ranking contacts, limited to 10.

-- NOTES
-- 2/24 (ming): Requires semi join support https://github.com/paradedb/paradedb/pull/4226 for join custom scan pushdown
-- HOWEVER even with this PR, the second query is not pushed down because the row estimates end up partitioning contact_list
-- instead of contacts_companies_combined_full, and our current broadcast join approach requires the left table be partitioned
-- I tried hacking something together to force a semi join with the contact_list table partitioned, but it ended up slower than the
-- non pushed down version

SET work_mem = '1GB';
SET max_parallel_workers_per_gather = 8;

SET paradedb.enable_join_custom_scan TO off;
EXPLAIN ANALYZE SELECT * FROM contacts_companies_combined_full
WHERE contact_id IN (
    SELECT ldf_id
    FROM contact_list
    WHERE list_id IN ('21430')
)
AND company_id @@@ pdb.all()
ORDER BY revenue_rank DESC, contact_id ASC
LIMIT 10;

SET paradedb.enable_join_custom_scan TO on;
EXPLAIN ANALYZE SELECT * FROM contacts_companies_combined_full
WHERE contact_id IN (
    SELECT ldf_id
    FROM contact_list
    WHERE list_id IN ('21430')
)
AND company_id @@@ pdb.all()
ORDER BY revenue_rank DESC, contact_id ASC
LIMIT 10;
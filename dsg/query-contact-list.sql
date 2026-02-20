-- Original Query: Filter by contact list (unsorted, full result set).
-- Converted to Top-K: Ordering by `revenue_rank` (fast field) descending to find the highest-ranking contacts, limited to 10.

PREPARE query_stmt AS
SELECT * FROM contacts_companies_combined_full
WHERE contact_id IN (
    SELECT ldf_id
    FROM contact_list
    WHERE list_id IN ('21430')
)
ORDER BY revenue_rank DESC, contact_id ASC
LIMIT 10;

EXPLAIN (ANALYZE, BUFFERS) EXECUTE query_stmt;

EXECUTE query_stmt;

DEALLOCATE query_stmt;
-- Original Query: Filter by company list (unsorted, full result set).
-- Converted to Top-K: Ordering by `revenue_rank` (fast field) descending to find the highest-ranking contacts, limited to 10.

SELECT * FROM contacts_companies_combined_full
WHERE company_id IN (
    SELECT ldf_id
    FROM company_list
    WHERE list_id IN ('2543')
)
ORDER BY revenue_rank DESC, contact_id ASC
LIMIT 10;

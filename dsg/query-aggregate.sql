-- Original Query: Top-K aggregation by `doc_count` descending.
-- Converted to Top-K: Flattened to return individual rows, ordering by `revenue_rank` (fast field) descending, limited to 10.

PREPARE query_stmt AS
SELECT *
FROM contacts_companies_combined_full
WHERE contact_id IN (
    SELECT ldf_id
    FROM contact_list
    WHERE list_id IN ('tjy3slfS5wk')
)
AND contact_id NOT IN (
    SELECT ldf_id
    FROM contact_list
    WHERE list_id IN ('loxSXiPQRww','SZWEZLQhwbE')
)
AND contact_id @@@ paradedb.boolean(
    must => ARRAY[
        paradedb.boolean(
            should => ARRAY[
                paradedb.boolean(
                    must => ARRAY[
                        paradedb.parse('contact_job_title:"Senior Programmer"'),
                        paradedb.parse('contact_job_details.job_function:"product management, research, & innovation"'),
                        paradedb.parse('contact_job_details.job_area:"software development & engineering"')
                    ])])
        ,
        paradedb.range(field => 'contact_id', range => '(0,)'::int8range)
    ]
)
ORDER BY revenue_rank DESC, contact_id ASC
LIMIT 10;

EXPLAIN (ANALYZE, BUFFERS) EXECUTE query_stmt;

EXECUTE query_stmt;

DEALLOCATE query_stmt;
-- Original Query: Top-K aggregation by `doc_count` descending.
-- Converted to Top-K: Flattened to return individual rows, ordering by `revenue_rank` (fast field) descending, limited to 10.

SET work_mem = '1GB';
SET max_parallel_workers_per_gather = 8;

-- NOTES
-- 2/25 (ming): We are not pushing this down, this shape not yet supported
-- 3/02 (stuhood): https://github.com/paradedb/paradedb/pull/4241 nets us a warning, but actual
--      execution requires support for anti joins.

SET paradedb.enable_join_custom_scan TO off;
EXPLAIN ANALYZE
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

SET paradedb.enable_join_custom_scan TO on;
EXPLAIN ANALYZE SELECT *
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

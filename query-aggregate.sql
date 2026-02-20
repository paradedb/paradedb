SET max_parallel_workers to 0;

SELECT
    contact_last_name,
    count(*) AS doc_count,
    count(DISTINCT company_id) AS company_count
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
        ),
        paradedb.range(field => 'contact_id', range => '(0,)'::int8range)
    ]
)
AND contact_last_name IS NOT NULL
GROUP BY contact_last_name
ORDER BY doc_count DESC
LIMIT 10;

SET work_mem = '1GB';
SET max_parallel_workers_per_gather = 8;

-- NOTES
-- 2/25 (ming): Join scan pushdown not supported yet
--      WARNING:  JoinScan not used: only INNER/SEMI JOIN is currently supported, got 5 (tables: contacts_companies_combined_full, csa_not_exists)
--      5 is an ANTI JOIN
-- 3/02 (stuhood): https://github.com/paradedb/paradedb/pull/4241 nets us an improved warning, but actual
--      execution requires support for anti joins.

SET paradedb.enable_join_custom_scan TO off;
EXPLAIN ANALYZE SELECT *
FROM contacts_companies_combined_full
WHERE
    -- Must have at least one specialty
    EXISTS (
        SELECT 1
        FROM company_specialties_autocomplete csa_exists
        WHERE csa_exists.company_id = contacts_companies_combined_full.company_id
    )
    -- Must NOT have the 'salesforce' specialty
    AND NOT EXISTS (
        SELECT 1
        FROM company_specialties_autocomplete csa_not_exists
        WHERE csa_not_exists.company_id = contacts_companies_combined_full.company_id
          AND csa_not_exists.unique_id @@@ paradedb.parse('speciality:salesforce')
    )
    AND contact_id @@@ paradedb.range(field => 'contact_id', range => '(0,)'::int8range)
ORDER BY revenue_rank DESC NULLS LAST, contact_id ASC
LIMIT 25;

SET paradedb.enable_join_custom_scan TO on;
EXPLAIN ANALYZE SELECT *
FROM contacts_companies_combined_full
WHERE
    -- Must have at least one specialty
    EXISTS (
        SELECT 1
        FROM company_specialties_autocomplete csa_exists
        WHERE csa_exists.company_id = contacts_companies_combined_full.company_id
    )
    -- Must NOT have the 'salesforce' specialty
    AND NOT EXISTS (
        SELECT 1
        FROM company_specialties_autocomplete csa_not_exists
        WHERE csa_not_exists.company_id = contacts_companies_combined_full.company_id
          AND csa_not_exists.unique_id @@@ paradedb.parse('speciality:salesforce')
    )
    AND contact_id @@@ paradedb.range(field => 'contact_id', range => '(0,)'::int8range)
ORDER BY revenue_rank DESC NULLS LAST, contact_id ASC
LIMIT 25;

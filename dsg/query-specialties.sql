PREPARE query_stmt AS
SELECT *
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

EXPLAIN (ANALYZE, BUFFERS) EXECUTE query_stmt;

EXECUTE query_stmt;

DEALLOCATE query_stmt;
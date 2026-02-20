SELECT *
FROM contacts_companies_combined_full
WHERE
    company_id IN (
        SELECT company_id
        FROM company_specialties_autocomplete
        WHERE company_id NOT IN (
            SELECT company_id
            FROM company_specialties_autocomplete
            WHERE unique_id @@@ paradedb.parse('speciality:salesforce')
        )
    )
    AND contact_id @@@ paradedb.range(field => 'contact_id', range => '(0,)'::int8range)
ORDER BY revenue_rank DESC NULLS LAST, contact_id ASC
LIMIT 25 OFFSET 0;
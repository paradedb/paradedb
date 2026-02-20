SELECT *
FROM contacts_companies_combined_full
WHERE
    company_id IN (
        SELECT company_id
        FROM company_tech_install_autocomplete
        WHERE unique_id @@@ paradedb.parse('technology_name:IN ["salesforce"]')
    )
    AND contact_id @@@ paradedb.range(field => 'contact_id', range => '(0,)'::int8range)
ORDER BY contact_first_name ASC NULLS LAST, contact_id ASC
LIMIT 25;
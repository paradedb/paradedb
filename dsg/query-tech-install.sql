SELECT *
FROM contacts_companies_combined_full
WHERE contact_id @@@ paradedb.boolean(must => ARRAY[
paradedb.boolean(must => ARRAY[paradedb.term_set(
    'company_id',
    (
        SELECT coalesce(array_agg(company_id), '{}')
        FROM (
            SELECT company_id FROM company_tech_install_autocomplete WHERE unique_id @@@ paradedb.parse('technology_name:IN ["salesforce"]') GROUP BY company_id
        )
    )
)]),
paradedb.boolean(must => ARRAY[paradedb.range(field => 'contact_id', range => '(0,)'::int8range)])])
ORDER BY contact_first_name ASC NULLS LAST, contact_id ASC
LIMIT 25 OFFSET 0;

SELECT *
FROM contacts_companies_combined_full
WHERE contact_id @@@ paradedb.boolean(must => ARRAY[
paradedb.boolean(must => ARRAY[paradedb.boolean(must => ARRAY[paradedb.boolean(must => ARRAY[paradedb.term_set(
    'company_id',
    (
        SELECT ARRAY(
            SELECT company_id FROM company_specialties_autocomplete WHERE unique_id @@@ (paradedb.boolean(must_not => 
                ARRAY[
                    paradedb.term_set(
                        'company_id',
                        (
                            SELECT ARRAY(
                                SELECT company_id FROM company_specialties_autocomplete WHERE unique_id @@@ (paradedb.boolean(should => ARRAY[paradedb.parse('speciality:salesforce')])) GROUP BY company_id
                            )
                        )
                    )
                ], 
                must => ARRAY[paradedb.all()]
            )) GROUP BY company_id
        )
    )
)])])]),
paradedb.boolean(must => ARRAY[paradedb.range(field => 'contact_id', range => '(0,)'::int8range)])])
ORDER BY revenue_rank DESC NULLS LAST, contact_id ASC
LIMIT 25 OFFSET 0;

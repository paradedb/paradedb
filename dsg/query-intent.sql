SELECT *
FROM contacts_companies_combined_full
WHERE contact_id @@@ paradedb.boolean(must => ARRAY[
paradedb.boolean(must => ARRAY[paradedb.term_set(
    'company_id',
    (
        SELECT coalesce(array_agg(company_id), '{}')
        FROM (
            SELECT company_id FROM company_intent_autocomplete WHERE unique_id @@@ paradedb.parse('intent_topic:IN ["pre-employment & employee testing"]') AND (unique_id @@@ paradedb.range('score', int4range(1, 100, '()'))) GROUP BY company_id
        )
    )
)]),
paradedb.boolean(must => ARRAY[paradedb.range(field => 'contact_id', range => '(0,)'::int8range)])])
ORDER BY contact_first_name ASC NULLS LAST, contact_id ASC
LIMIT 25 OFFSET 0;

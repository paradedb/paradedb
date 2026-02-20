PREPARE query_stmt AS
SELECT *
FROM contacts_companies_combined_full
WHERE
    company_id IN (
        SELECT company_id
        FROM company_intent_autocomplete
        WHERE unique_id @@@ paradedb.parse('intent_topic:IN ["pre-employment & employee testing"]')
          AND unique_id @@@ paradedb.range('score', int4range(1, 100, '()'))
    )
    AND contact_id @@@ paradedb.range(field => 'contact_id', range => '(0,)'::int8range)
ORDER BY contact_first_name ASC NULLS LAST, contact_id ASC
LIMIT 25;

EXPLAIN (ANALYZE, BUFFERS) EXECUTE query_stmt;

EXECUTE query_stmt;

DEALLOCATE query_stmt;

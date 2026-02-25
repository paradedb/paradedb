SET work_mem = '1GB';
SET max_parallel_workers_per_gather = 8;

-- NOTES
-- 2/25 (ming): Requires semi join support https://github.com/paradedb/paradedb/pull/4226 for join custom scan pushdown
-- Big win on this query! 2s to 300ms

SET paradedb.enable_join_custom_scan TO off;
EXPLAIN ANALYZE SELECT *
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

SET paradedb.enable_join_custom_scan TO on;
EXPLAIN ANALYZE SELECT *
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

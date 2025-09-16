-- Test file to investigate query slowdown with range fields
-- This test specifically examines why a query with LIMIT is not using TopNScanExecState

-- Setup
\i common/common_setup.sql

-- Create test table with a structure containing various field types
DROP TABLE IF EXISTS records;
CREATE TABLE records (
    id SERIAL PRIMARY KEY,
    tenant_id TEXT,
    source_id TEXT,
    recipient_id TEXT,
    currency_code TEXT,
    recipient_name TEXT,
    reference_data TEXT,
    additional_info TEXT,
    flow_type TEXT,
    is_active BOOLEAN,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    routing_code TEXT,
    value_range NUMRANGE,
    time_period TSTZRANGE,
    state TEXT,
    metadata JSONB,
    notes TEXT,
    version_num INTEGER,
    process_method TEXT,
    origin_source TEXT,
    filter_config JSONB,
    system_name TEXT,
    link_count1 INTEGER,
    link_count2 INTEGER,
    amount_value NUMERIC,
    processed_amount NUMERIC,
    rule_id TEXT,
    tags TEXT[],
    external_id TEXT,
    removed_at TIMESTAMP
);

-- Insert test data
INSERT INTO records (
    tenant_id, source_id, notes, is_active,
    time_period, removed_at, currency_code, state, flow_type
)
SELECT
    'tenant-' || (i % 10),  -- 10 different tenants
    'source-' || (i % 100),  -- 100 different source IDs
    CASE WHEN i % 5 = 0 THEN 'check payment'
         WHEN i % 5 = 1 THEN 'check deposit'
         WHEN i % 5 = 2 THEN 'wire transfer'
         WHEN i % 5 = 3 THEN 'ach payment'
         ELSE 'credit card payment'
    END,
    i % 3 = 0,  -- 1/3 are is_active=true
    tstzrange(
        '2023-01-01'::timestamptz + ((i % 365) || ' days')::interval,
        '2023-01-01'::timestamptz + ((i % 365) || ' days')::interval + '1 day'::interval
    ),
    CASE WHEN i % 7 = 0 THEN '2023-06-01'::timestamp + ((i % 30) || ' days')::interval ELSE NULL END,
    CASE WHEN i % 4 = 0 THEN 'USD' WHEN i % 4 = 1 THEN 'EUR' WHEN i % 4 = 2 THEN 'GBP' ELSE 'JPY' END,
    CASE WHEN i % 4 = 0 THEN 'pending' WHEN i % 4 = 1 THEN 'completed' WHEN i % 4 = 2 THEN 'failed' ELSE NULL END,
    CASE WHEN i % 2 = 0 THEN 'inbound' ELSE 'outbound' END
FROM generate_series(1, 10000) i;

-- Create a search index with various field types
CREATE INDEX records_search_idx ON records
USING bm25 (
    id, notes, flow_type, tenant_id, state, source_id, 
    is_active, metadata, value_range, time_period, created_at, updated_at, 
    tags, process_method, currency_code, recipient_id, 
    rule_id, link_count2, removed_at, amount_value
) WITH (
    key_field = 'id',
    text_fields = '{
        "notes": { 
            "normalizer": "lowercase", 
            "tokenizer": { "max_gram": 3, "min_gram": 3, "prefix_only": false, "type": "ngram" }
        },
        "flow_type": { "fast": true, "tokenizer": {"type": "keyword"} },
        "tenant_id": { "tokenizer": {"type": "keyword"} },
        "recipient_id": { "tokenizer": {"type": "keyword"} },
        "source_id": { "tokenizer": {"type": "keyword"} },
        "rule_id": { "tokenizer": {"type": "keyword"} },
        "tags": { "fast": true, "tokenizer": {"type": "keyword"} },
        "state": { "fast": true, "tokenizer": {"type": "keyword"} },
        "currency_code": { "tokenizer": {"type": "keyword"} },
        "process_method": { "fast": true, "tokenizer": {"type": "keyword"} }
    }',
    json_fields = '{ 
        "metadata": { "fast": true, "normalizer": "lowercase", "tokenizer": { "type": "raw" } }, 
        "metadata_words": { "fast": true, "normalizer": "lowercase", "tokenizer": { "type": "default" }, "column": "metadata" } 
    }'
);

\echo '======== EXECUTION METHOD TESTS ========'
\echo 'Tests to identify when TopNScanExecState vs NormalScanExecState is used'

-- Test 1: Simple query with LIMIT (should use TopNScanExecState)
\echo 'Test 1: Simple query with LIMIT (should use TopNScanExecState)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, notes, tenant_id
FROM records
WHERE notes @@@ 'check'
ORDER BY id
LIMIT 25;

-- Test 2: Query with complex conditions
\echo 'Test 2: Complex query with multiple conditions (reproducing issue)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, tenant_id, source_id, notes, is_active, time_period
FROM records
WHERE tenant_id = 'tenant-1'
  AND is_active = TRUE
  AND NOT (source_id @@@ 'IN [source-1 source-2 source-3]')
  AND (id @@@ paradedb.match('notes', 'check', conjunction_mode => true, 
       tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false)))
  AND (NOT id @@@ paradedb.exists('removed_at'))
  AND (id @@@ paradedb.all())
ORDER BY time_period DESC
LIMIT 25;

-- Test 3: Same query without paradedb.all() operator
\echo 'Test 3: Without paradedb.all() operator'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, tenant_id, source_id, notes, is_active, time_period
FROM records
WHERE tenant_id = 'tenant-1'
  AND is_active = TRUE
  AND NOT (source_id @@@ 'IN [source-1 source-2 source-3]')
  AND (id @@@ paradedb.match('notes', 'check', conjunction_mode => true, 
       tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false)))
  AND (NOT id @@@ paradedb.exists('removed_at'))
ORDER BY time_period DESC
LIMIT 25;

-- Test 4: Testing the impact of paradedb.all() alone
\echo 'Test 4: Testing paradedb.all() by itself'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, notes
FROM records
WHERE id @@@ paradedb.all()
ORDER BY id
LIMIT 25;

-- Test 5: Testing NOT with paradedb.exists
\echo 'Test 5: Testing NOT with paradedb.exists'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, notes
FROM records
WHERE NOT id @@@ paradedb.exists('removed_at')
ORDER BY id
LIMIT 25;

-- Test 6: Testing IN negation with @@@
\echo 'Test 6: Testing IN negation with @@@'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, notes
FROM records
WHERE NOT source_id @@@ 'IN [source-1 source-2 source-3]'
ORDER BY id
LIMIT 25;

-- Test 7: Testing simplified complex query - removing one component at a time
\echo 'Test 7: Simplified complex query - only basic conditions'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, tenant_id, notes
FROM records
WHERE tenant_id = 'tenant-1'
  AND is_active = TRUE
  AND id @@@ paradedb.match('notes', 'check')
ORDER BY time_period DESC
LIMIT 25;

-- Test 8: Testing with each complex component isolated
\echo 'Test 8: Component isolation - paradedb.match'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, notes
FROM records
WHERE id @@@ paradedb.match('notes', 'check', conjunction_mode => true, 
       tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false))
LIMIT 25;

-- Test 9: Query with all columns
\echo 'Test 9: Full query with all 31 columns'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, tenant_id, source_id, recipient_id, currency_code, 
       recipient_name, reference_data, additional_info, flow_type,
       is_active, created_at, updated_at, routing_code, 
       value_range, time_period, state, metadata, notes, version_num,
       process_method, origin_source, filter_config, 
       system_name, link_count1, link_count2,
       amount_value, processed_amount, rule_id,
       tags, external_id, removed_at
FROM records
WHERE tenant_id = 'tenant-1'
  AND is_active = TRUE
  AND NOT (source_id @@@ 'IN [source-1 source-2 source-3]')
  AND (id @@@ paradedb.match('notes', 'check', conjunction_mode => true, 
       tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false)))
  AND (NOT id @@@ paradedb.exists('removed_at'))
  AND (id @@@ paradedb.all())
ORDER BY time_period DESC
LIMIT 25;

-- Test 10: Fast-field only solution - selecting only indexed fast fields
\echo 'Test 10: Possible solution - selecting only fast fields'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, flow_type, state, tags, process_method
FROM records
WHERE tenant_id = 'tenant-1'
  AND is_active = TRUE
  AND (id @@@ paradedb.match('notes', 'check'))
ORDER BY time_period DESC
LIMIT 25;

-- Test 11: ORDER BY on column containing NULL.
\echo 'Test 11: ORDER BY on column containing NULL.'
EXPLAIN (ANALYZE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF)
SELECT state, id
FROM records
WHERE id @@@ paradedb.all()
  AND state IS NULL
ORDER BY state, id
LIMIT 10000;

-- Cleanup
DROP INDEX IF EXISTS records_search_idx;
DROP TABLE IF EXISTS records;

\i common/common_cleanup.sql 

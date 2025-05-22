-- Test file to investigate customer query slowdown
-- This test specifically examines why a query with LIMIT is not using TopNScanExecState

-- Setup
\i common/mixedff_advanced_setup.sql

-- Create test table with structure similar to customer's expected_payments
DROP TABLE IF EXISTS payments;
CREATE TABLE payments (
    id SERIAL PRIMARY KEY,
    organization_id TEXT,
    internal_account_id TEXT,
    counterparty_id TEXT,
    currency TEXT,
    counterparty_name TEXT,
    statement_descriptor TEXT,
    remittance_information TEXT,
    direction TEXT,
    live_mode BOOLEAN,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    counterparty_bank_routing_number TEXT,
    amount_range NUMRANGE,
    date_range TSTZRANGE,
    status TEXT,
    metadata_json JSONB,
    description TEXT,
    lock_version INTEGER,
    reconciliation_method TEXT,
    creation_source TEXT,
    reconciliation_filters JSONB,
    reconciliation_system TEXT,
    financial_entity_link_count INTEGER,
    entity_link_count INTEGER,
    directional_amount NUMERIC,
    amount_reconciled NUMERIC,
    reconciliation_rule_id TEXT,
    payment_types TEXT[],
    transaction_id TEXT,
    discarded_at TIMESTAMP
);

-- Insert test data
INSERT INTO payments (
    organization_id, internal_account_id, description, live_mode,
    date_range, discarded_at, currency, status, direction
)
SELECT
    'org-' || (i % 10),  -- 10 different organizations
    'account-' || (i % 100),  -- 100 different account IDs
    CASE WHEN i % 5 = 0 THEN 'check payment'
         WHEN i % 5 = 1 THEN 'check deposit'
         WHEN i % 5 = 2 THEN 'wire transfer'
         WHEN i % 5 = 3 THEN 'ach payment'
         ELSE 'credit card payment'
    END,
    i % 3 = 0,  -- 1/3 are live_mode=true
    tstzrange(
        '2023-01-01'::timestamptz + ((i % 365) || ' days')::interval,
        '2023-01-01'::timestamptz + ((i % 365) || ' days')::interval + '1 day'::interval
    ),
    CASE WHEN i % 7 = 0 THEN '2023-06-01'::timestamp + ((i % 30) || ' days')::interval ELSE NULL END,
    CASE WHEN i % 4 = 0 THEN 'USD' WHEN i % 4 = 1 THEN 'EUR' WHEN i % 4 = 2 THEN 'GBP' ELSE 'JPY' END,
    CASE WHEN i % 3 = 0 THEN 'pending' WHEN i % 3 = 1 THEN 'completed' ELSE 'failed' END,
    CASE WHEN i % 2 = 0 THEN 'inbound' ELSE 'outbound' END
FROM generate_series(1, 10000) i;

-- Create a search index similar to the customer's
CREATE INDEX payments_search_idx ON payments
USING bm25 (
    id, description, direction, organization_id, status, internal_account_id, 
    live_mode, metadata_json, amount_range, date_range, created_at, updated_at, 
    payment_types, reconciliation_method, currency, counterparty_id, 
    reconciliation_rule_id, entity_link_count, discarded_at, directional_amount
) WITH (
    key_field = 'id',
    text_fields = '{
        "description": { 
            "normalizer": "lowercase", 
            "tokenizer": { "max_gram": 3, "min_gram": 3, "prefix_only": false, "type": "ngram" }
        },
        "direction": { "fast": true, "tokenizer": {"type": "keyword"} },
        "organization_id": { "tokenizer": {"type": "keyword"} },
        "counterparty_id": { "tokenizer": {"type": "keyword"} },
        "internal_account_id": { "tokenizer": {"type": "keyword"} },
        "reconciliation_rule_id": { "tokenizer": {"type": "keyword"} },
        "payment_types": { "fast": true, "tokenizer": {"type": "keyword"} },
        "status": { "fast": true, "tokenizer": {"type": "keyword"} },
        "currency": { "tokenizer": {"type": "keyword"} },
        "reconciliation_method": { "fast": true, "tokenizer": {"type": "keyword"} }
    }',
    numeric_fields = '{"entity_link_count":{}, "directional_amount":{}}',
    boolean_fields = '{"live_mode":{}}',
    json_fields = '{ 
        "metadata_json": { "fast": true, "normalizer": "lowercase", "tokenizer": { "type": "raw" } }, 
        "metadata_json_words": { "fast": true, "normalizer": "lowercase", "tokenizer": { "type": "default" }, "column": "metadata_json" } 
    }',
    range_fields = '{"amount_range":{"fast":true},"date_range":{"fast":true}}',
    datetime_fields = '{"created_at":{}, "discarded_at":{}, "updated_at":{}}'
);

\echo '======== EXECUTION METHOD TESTS ========'
\echo 'Tests to identify when TopNScanExecState vs NormalScanExecState is used'

-- Test 1: Simple query with LIMIT (should use TopNScanExecState)
\echo 'Test 1: Simple query with LIMIT (should use TopNScanExecState)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, organization_id
FROM payments
WHERE description @@@ 'check'
ORDER BY id
LIMIT 25;

-- Test 2: Query similar to customer's with complex conditions
\echo 'Test 2: Complex query with multiple conditions (reproducing customer issue)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, organization_id, internal_account_id, description, live_mode, date_range
FROM payments
WHERE organization_id = 'org-1'
  AND live_mode = TRUE
  AND NOT (internal_account_id @@@ 'IN [account-1 account-2 account-3]')
  AND (id @@@ paradedb.match('description', 'check', conjunction_mode => true, 
       tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false)))
  AND (NOT id @@@ paradedb.exists('discarded_at'))
  AND (id @@@ paradedb.all())
ORDER BY date_range DESC
LIMIT 25;

-- Test 3: Same query without paradedb.all() operator
\echo 'Test 3: Without paradedb.all() operator'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, organization_id, internal_account_id, description, live_mode, date_range
FROM payments
WHERE organization_id = 'org-1'
  AND live_mode = TRUE
  AND NOT (internal_account_id @@@ 'IN [account-1 account-2 account-3]')
  AND (id @@@ paradedb.match('description', 'check', conjunction_mode => true, 
       tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false)))
  AND (NOT id @@@ paradedb.exists('discarded_at'))
ORDER BY date_range DESC
LIMIT 25;

-- Test 4: Testing the impact of paradedb.all() alone
\echo 'Test 4: Testing paradedb.all() by itself'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description
FROM payments
WHERE id @@@ paradedb.all()
ORDER BY id
LIMIT 25;

-- Test 5: Testing NOT with paradedb.exists
\echo 'Test 5: Testing NOT with paradedb.exists'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description
FROM payments
WHERE NOT id @@@ paradedb.exists('discarded_at')
ORDER BY id
LIMIT 25;

-- Test 6: Testing IN negation with @@@
\echo 'Test 6: Testing IN negation with @@@'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description
FROM payments
WHERE NOT internal_account_id @@@ 'IN [account-1 account-2 account-3]'
ORDER BY id
LIMIT 25;

-- Test 7: Testing simplified complex query - removing one component at a time
\echo 'Test 7: Simplified complex query - only basic conditions'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, organization_id, description
FROM payments
WHERE organization_id = 'org-1'
  AND live_mode = TRUE
  AND id @@@ paradedb.match('description', 'check')
ORDER BY date_range DESC
LIMIT 25;

-- Test 8: Testing the original customer query with each complex component isolated
\echo 'Test 8: Component isolation - paradedb.match'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description
FROM payments
WHERE id @@@ paradedb.match('description', 'check', conjunction_mode => true, 
       tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false))
LIMIT 25;

-- Test 9: Exact reproduction of customer's query with all columns
\echo 'Test 9: Full customer query with all 31 columns'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, organization_id, internal_account_id, counterparty_id, currency, 
       counterparty_name, statement_descriptor, remittance_information, direction,
       live_mode, created_at, updated_at, counterparty_bank_routing_number, 
       amount_range, date_range, status, metadata_json, description, lock_version,
       reconciliation_method, creation_source, reconciliation_filters, 
       reconciliation_system, financial_entity_link_count, entity_link_count,
       directional_amount, amount_reconciled, reconciliation_rule_id,
       payment_types, transaction_id, discarded_at
FROM payments
WHERE organization_id = 'org-1'
  AND live_mode = TRUE
  AND NOT (internal_account_id @@@ 'IN [account-1 account-2 account-3]')
  AND (id @@@ paradedb.match('description', 'check', conjunction_mode => true, 
       tokenizer => paradedb.tokenizer('ngram', min_gram => 3, max_gram => 3, prefix_only => false)))
  AND (NOT id @@@ paradedb.exists('discarded_at'))
  AND (id @@@ paradedb.all())
ORDER BY date_range DESC
LIMIT 25;

-- Test 10: Fast-field only solution - selecting only indexed fast fields
\echo 'Test 10: Possible solution - selecting only fast fields'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, direction, status, payment_types, reconciliation_method
FROM payments
WHERE organization_id = 'org-1'
  AND live_mode = TRUE
  AND (id @@@ paradedb.match('description', 'check'))
ORDER BY date_range DESC
LIMIT 25;

-- Cleanup
DROP INDEX IF EXISTS payments_search_idx;
DROP TABLE IF EXISTS payments;

\i common/mixedff_advanced_cleanup.sql 

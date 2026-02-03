-- =====================================================================
-- Test Suite for DateTime GROUP BY Aggregation
-- =====================================================================
-- This test verifies that GROUP BY on DateTime fields works correctly
-- with the ParadeDB aggregate custom scan.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test Data Setup
-- =====================================================================

DROP TABLE IF EXISTS transactions CASCADE;
CREATE TABLE transactions (
    id SERIAL PRIMARY KEY,
    organization_id TEXT,
    live_mode BOOLEAN,
    discarded_at TIMESTAMP,
    internal_account_id TEXT,
    unledgered_amount NUMERIC,
    as_of_date DATE,
    description TEXT
);

INSERT INTO transactions (organization_id, live_mode, discarded_at, internal_account_id, unledgered_amount, as_of_date, description) VALUES
    ('org1', TRUE, NULL, 'account1', 100, '2025-12-26', 'Transaction 1a'),
    ('org1', TRUE, NULL, 'account1', 150, '2025-12-26', 'Transaction 1b'),
    ('org1', TRUE, NULL, 'account1', 200, '2025-12-27', 'Transaction 2a'),
    ('org1', TRUE, NULL, 'account1', 250, '2025-12-27', 'Transaction 2b'),
    ('org1', TRUE, NULL, 'account1', 275, '2025-12-27', 'Transaction 2c'),
    ('org1', TRUE, NULL, 'account1', 300, '2025-12-28', 'Transaction 3'),
    ('org1', TRUE, NULL, 'account1', 0, '2025-12-29', 'Zero amount'),
    ('org1', FALSE, NULL, 'account1', 400, '2025-12-30', 'Not live a'),
    ('org1', FALSE, NULL, 'account1', 450, '2025-12-30', 'Not live b'),
    ('org1', TRUE, '2025-01-01', 'account1', 500, '2025-12-31', 'Discarded'),
    ('org1', TRUE, NULL, 'account1', 600, NULL, 'Null date');

CREATE INDEX transactions_search_index ON transactions
USING bm25 (id, organization_id, live_mode, discarded_at, internal_account_id, unledgered_amount, as_of_date, description)
WITH (
    key_field='id',
    text_fields='{"description": {}, "organization_id": {"fast": true}, "internal_account_id": {"fast": true}}',
    boolean_fields='{"live_mode": {"fast": true}}',
    numeric_fields='{"unledgered_amount": {"fast": true}}',
    datetime_fields='{"as_of_date": {"fast": true}, "discarded_at": {"fast": true}}'
);

-- =====================================================================
-- Test 1: Simple GROUP BY on DateTime field
-- =====================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS count_all,
    transactions.as_of_date
FROM transactions
WHERE transactions.id @@@ paradedb.all()
GROUP BY transactions.as_of_date;

SELECT 
    COUNT(*) AS count_all,
    transactions.as_of_date
FROM transactions
WHERE transactions.id @@@ paradedb.all()
GROUP BY transactions.as_of_date
ORDER BY as_of_date NULLS LAST;

-- =====================================================================
-- Test 2: GROUP BY DateTime with BETWEEN range filter
-- =====================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS count_all,
    transactions.as_of_date
FROM transactions
WHERE transactions.organization_id = 'org1'
    AND transactions.live_mode = TRUE
    AND transactions.discarded_at IS NULL
    AND transactions.unledgered_amount != 0
    AND transactions.id @@@ paradedb.all()
    AND transactions.as_of_date BETWEEN '2025-12-25' AND '2026-02-01'
GROUP BY transactions.as_of_date;

SELECT 
    COUNT(*) AS count_all,
    transactions.as_of_date
FROM transactions
WHERE transactions.organization_id = 'org1'
    AND transactions.live_mode = TRUE
    AND transactions.discarded_at IS NULL
    AND transactions.unledgered_amount != 0
    AND transactions.id @@@ paradedb.all()
    AND transactions.as_of_date BETWEEN '2025-12-25' AND '2026-02-01'
GROUP BY transactions.as_of_date
ORDER BY as_of_date;

-- =====================================================================
-- Test 3: GROUP BY DateTime with < operator
-- =====================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS count_all,
    transactions.as_of_date
FROM transactions
WHERE transactions.id @@@ paradedb.all()
    AND transactions.as_of_date < '2025-12-29'
GROUP BY transactions.as_of_date;

SELECT 
    COUNT(*) AS count_all,
    transactions.as_of_date
FROM transactions
WHERE transactions.id @@@ paradedb.all()
    AND transactions.as_of_date < '2025-12-29'
GROUP BY transactions.as_of_date
ORDER BY as_of_date;

-- =====================================================================
-- Test 4: GROUP BY DateTime with > operator
-- =====================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    COUNT(*) AS count_all,
    transactions.as_of_date
FROM transactions
WHERE transactions.id @@@ paradedb.all()
    AND transactions.as_of_date > '2025-12-28'
GROUP BY transactions.as_of_date;

SELECT 
    COUNT(*) AS count_all,
    transactions.as_of_date
FROM transactions
WHERE transactions.id @@@ paradedb.all()
    AND transactions.as_of_date > '2025-12-28'
GROUP BY transactions.as_of_date
ORDER BY as_of_date;

-- =====================================================================
-- Test 5: MIN/MAX aggregates on DateTime field
-- =====================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    MIN(as_of_date) AS min_date,
    MAX(as_of_date) AS max_date
FROM transactions
WHERE transactions.id @@@ paradedb.all()
    AND transactions.live_mode = TRUE;

SELECT 
    MIN(as_of_date) AS min_date,
    MAX(as_of_date) AS max_date
FROM transactions
WHERE transactions.id @@@ paradedb.all()
    AND transactions.live_mode = TRUE;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE IF EXISTS transactions CASCADE;
RESET paradedb.enable_aggregate_custom_scan;

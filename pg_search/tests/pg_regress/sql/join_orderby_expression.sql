-- Tests for JoinScan with ORDER BY expressions that are order-preserving
-- wrappers around bare columns (e.g. id + 0, id - 0, id * 1, id / 1).
-- Regression test for https://github.com/paradedb/paradedb/issues/4754

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS companies CASCADE;
DROP TABLE IF EXISTS funding_rounds CASCADE;

CREATE TABLE companies (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT
);

CREATE TABLE funding_rounds (
    id INTEGER PRIMARY KEY,
    company_id INTEGER,
    amount INTEGER,
    round_type TEXT
);

INSERT INTO companies (id, name, description) VALUES
(1, 'TechStartup', 'A technology startup building innovative solutions'),
(2, 'DataCorp', 'Data analytics and machine learning company'),
(3, 'CloudNet', 'Cloud networking and infrastructure provider'),
(4, 'AIVentures', 'Artificial intelligence research and development'),
(5, 'WebScale', 'Web-scale distributed systems company');

INSERT INTO funding_rounds (id, company_id, amount, round_type) VALUES
(101, 1, 1000000, 'seed'),
(102, 1, 5000000, 'series_a'),
(103, 2, 2000000, 'seed'),
(104, 3, 10000000, 'series_b'),
(105, 4, 500000, 'seed'),
(106, 5, 3000000, 'series_a');

CREATE INDEX companies_bm25_idx ON companies USING bm25 (id, name, description)
WITH (key_field = 'id');

CREATE INDEX funding_rounds_bm25_idx ON funding_rounds
USING bm25 (id, company_id, amount, (round_type::pdb.literal))
WITH (
    key_field = 'id',
    numeric_fields = '{"company_id": {"fast": true}, "amount": {"fast": true}}'
);

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: Baseline — JoinScan activates with bare ORDER BY c.id DESC
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id DESC
LIMIT 10;

SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id DESC
LIMIT 10;

-- =============================================================================
-- TEST 2: JoinScan should activate with ORDER BY c.id + 0 DESC
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id + 0 DESC
LIMIT 10;

SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id + 0 DESC
LIMIT 10;

-- =============================================================================
-- TEST 3: Results must match between bare column and expression form
-- =============================================================================

-- Bare column form
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id DESC
LIMIT 10;

-- Expression form (id + 0) — must return identical rows
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id + 0 DESC
LIMIT 10;

-- =============================================================================
-- TEST 4: ORDER BY c.id - 0 DESC (subtraction with zero on right)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id - 0 DESC
LIMIT 10;

SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id - 0 DESC
LIMIT 10;

-- =============================================================================
-- TEST 5: ORDER BY c.id * 1 DESC (multiplication by one)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id * 1 DESC
LIMIT 10;

SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id * 1 DESC
LIMIT 10;

-- =============================================================================
-- TEST 6: ORDER BY c.id / 1 DESC (division by one)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id / 1 DESC
LIMIT 10;

SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id / 1 DESC
LIMIT 10;

-- =============================================================================
-- TEST 7: Nested wrappers — (c.id + 0)::int4 should still unwrap
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY (c.id + 0)::int4 DESC
LIMIT 10;

SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY (c.id + 0)::int4 DESC
LIMIT 10;

-- =============================================================================
-- TEST 8: Non-identity expression should NOT activate JoinScan
-- ORDER BY c.id + 1 DESC — this is NOT order-preserving identity
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id + 1 DESC
LIMIT 10;

-- =============================================================================
-- TEST 9: 0 - c.id should NOT unwrap (inverts ordering)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY 0 - c.id DESC
LIMIT 10;

-- =============================================================================
-- TEST 10: Var + Var should NOT unwrap (not a Const)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id + c.id DESC
LIMIT 10;

-- =============================================================================
-- TEST 11: Using === operator alongside @@@
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type === 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id + 0 DESC
LIMIT 10;

SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type === 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id + 0 DESC
LIMIT 10;

-- =============================================================================
-- TEST 12: Cross-type expression should NOT unwrap (different operator OID)
-- c.id is int4, 0::bigint makes this int48pl (OID not in whitelist)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type === 'seed'
)
AND c.description @@@ 'technology'
ORDER BY c.id + 0::bigint DESC
LIMIT 10;

-- =============================================================================
-- TEST 13: Production-shape query — multiple IN subqueries with NOT EXISTS
-- =============================================================================

DROP TABLE IF EXISTS orders CASCADE;

CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    company_id INTEGER,
    status TEXT,
    total INTEGER
);

INSERT INTO orders (id, company_id, status, total) VALUES
(201, 1, 'completed', 5000),
(202, 2, 'pending', 3000),
(203, 1, 'cancelled', 1000),
(204, 3, 'completed', 8000),
(205, 4, 'completed', 2000);

CREATE INDEX orders_bm25_idx ON orders USING bm25 (id, company_id, (status::pdb.literal), total)
WITH (
    key_field = 'id',
    numeric_fields = '{"company_id": {"fast": true}, "total": {"fast": true}}'
);

-- Complex query with multiple IN subqueries and NOT EXISTS
-- Result correctness: bare column vs expression form must match.

SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
    AND NOT EXISTS (
        SELECT 1 FROM orders o
        WHERE o.company_id = fr.company_id
        AND o.status @@@ 'cancelled'
    )
)
AND c.description @@@ 'technology OR intelligence'
ORDER BY c.id DESC
LIMIT 10;

SELECT c.id, c.name
FROM companies c
WHERE c.id IN (
    SELECT fr.company_id
    FROM funding_rounds fr
    WHERE fr.round_type @@@ 'seed'
    AND NOT EXISTS (
        SELECT 1 FROM orders o
        WHERE o.company_id = fr.company_id
        AND o.status @@@ 'cancelled'
    )
)
AND c.description @@@ 'technology OR intelligence'
ORDER BY c.id + 0 DESC
LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS funding_rounds CASCADE;
DROP TABLE IF EXISTS companies CASCADE;

RESET paradedb.enable_join_custom_scan;
RESET max_parallel_workers_per_gather;
RESET enable_indexscan;

-- Regression test for GitHub issue #4751:
-- Join pushdown not applied when ORDER BY contains IS NULL expression.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS test_people CASCADE;
DROP TABLE IF EXISTS test_companies CASCADE;

CREATE TABLE test_companies (
    id INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE test_people (
    id INTEGER PRIMARY KEY,
    company_id INTEGER
);

INSERT INTO test_companies (id, name) VALUES
(101, 'Acme'), (102, 'Globex'), (103, NULL), (104, 'Initech'), (105, NULL);

INSERT INTO test_people (id, company_id) VALUES
(201, 101), (202, 101), (203, 102), (204, 104);

CREATE INDEX test_companies_bm25 ON test_companies USING bm25 (id, name)
    WITH (key_field = 'id', text_fields = '{"name": {"fast": true}}');
CREATE INDEX test_people_bm25 ON test_people USING bm25 (id, company_id)
    WITH (key_field = 'id', numeric_fields = '{"company_id": {"fast": true}}');

ANALYZE test_companies;
ANALYZE test_people;

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: ORDER BY col IS NULL ASC should get join pushdown
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id
FROM test_companies AS c
JOIN test_people AS p ON p.company_id = c.id
WHERE c.id @@@ paradedb.all()
ORDER BY c.name IS NULL ASC, c.name ASC, c.id ASC
LIMIT 26;

SELECT c.id
FROM test_companies AS c
JOIN test_people AS p ON p.company_id = c.id
WHERE c.id @@@ paradedb.all()
ORDER BY c.name IS NULL ASC, c.name ASC, c.id ASC
LIMIT 26;

-- =============================================================================
-- TEST 2: Verify results match fallback (non-JoinScan) path
-- =============================================================================

SET paradedb.enable_join_custom_scan = off;

SELECT c.id
FROM test_companies AS c
JOIN test_people AS p ON p.company_id = c.id
WHERE c.id @@@ paradedb.all()
ORDER BY c.name IS NULL ASC, c.name ASC, c.id ASC
LIMIT 26;

-- =============================================================================
-- TEST 3: ORDER BY col IS NOT NULL should also get join pushdown
-- =============================================================================

SET paradedb.enable_join_custom_scan = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.id
FROM test_companies AS c
JOIN test_people AS p ON p.company_id = c.id
WHERE c.id @@@ paradedb.all()
ORDER BY c.name IS NOT NULL ASC, c.name ASC, c.id ASC
LIMIT 26;

SELECT c.id
FROM test_companies AS c
JOIN test_people AS p ON p.company_id = c.id
WHERE c.id @@@ paradedb.all()
ORDER BY c.name IS NOT NULL ASC, c.name ASC, c.id ASC
LIMIT 26;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS test_people CASCADE;
DROP TABLE IF EXISTS test_companies CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;

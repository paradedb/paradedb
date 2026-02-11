-- Test for TopK dynamic filter pushdown through DataFusion
-- This test verifies that SortExec(TopK) propagates a DynamicFilterPhysicalExpr
-- down to PgSearchScan, enabling row pruning at the scan level for ORDER BY ... LIMIT queries.
-- Pruning occurs from two sources:
--   1) HashJoin dynamic filter: after building the hash table, HashJoinExec pushes
--      min/max bounds of join keys to the probe side, pruning rows that can't match.
--   2) TopK dynamic filter: after processing initial batches, SortExec pushes
--      the K-th threshold to leaf scans, pruning rows that can't make the top K.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;

CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    region TEXT
);

-- 5 suppliers. Only some will match search predicates in individual tests.
INSERT INTO suppliers (id, name, region) VALUES
(1, 'AlphaSupply', 'north america domestic shipping'),
(2, 'BetaGoods', 'europe international logistics'),
(3, 'GammaParts', 'asia pacific global trade'),
(4, 'DeltaCorp', 'south america regional distribution'),
(5, 'EpsilonTech', 'africa emerging market wireless');

CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER,
    price NUMERIC(10,2)
);

-- 30 products: all mention "premium" so they all match the search predicate.
-- supplier_id cycles 1-5, so each supplier has 6 products.
-- Prices spread from ~20 to ~300.
INSERT INTO products (id, name, description, supplier_id, price)
SELECT
    i,
    'Product ' || i,
    'premium quality item number ' || i || ' for professional use',
    (i % 5) + 1,
    round((10.0 + (i * 9.8))::numeric, 2)
FROM generate_series(1, 30) AS i;

CREATE INDEX products_bm25_idx ON products USING bm25 (id, name, description, supplier_id, price)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}, "price": {"fast": true}}');
CREATE INDEX suppliers_bm25_idx ON suppliers USING bm25 (id, name, region)
WITH (key_field = 'id');

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- TEST 1: EXPLAIN shows dynamic_filter=true on PgSearchScan with ORDER BY + LIMIT
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'premium'
ORDER BY p.id
LIMIT 3;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'premium'
ORDER BY p.id
LIMIT 3;

-- =============================================================================
-- TEST 2: Search predicate on the build side (suppliers) restricts join keys,
-- causing the HashJoin dynamic filter to prune probe-side (products) rows.
-- Only supplier 5 (EpsilonTech) matches 'wireless'. Products with supplier_id != 5
-- are pruned by the HashJoin dynamic filter on the probe side.
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE s.region @@@ 'wireless'
ORDER BY p.id
LIMIT 3;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE s.region @@@ 'wireless'
ORDER BY p.id
LIMIT 3;

-- =============================================================================
-- TEST 3: ORDER BY DESC + LIMIT
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'premium'
ORDER BY p.id DESC
LIMIT 2;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'premium'
ORDER BY p.id DESC
LIMIT 2;

-- =============================================================================
-- TEST 4: ORDER BY price + LIMIT (numeric sort column)
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'premium'
ORDER BY p.price ASC
LIMIT 2;

SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'premium'
ORDER BY p.price ASC
LIMIT 2;

-- =============================================================================
-- TEST 5: Both sides have search predicates with ORDER BY + LIMIT.
-- Only supplier 3 (GammaParts) matches 'global'. Products are further filtered.
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'premium' AND s.region @@@ 'global'
ORDER BY p.id
LIMIT 5;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'premium' AND s.region @@@ 'global'
ORDER BY p.id
LIMIT 5;

-- =============================================================================
-- TEST 6: Without LIMIT - no dynamic filter should appear
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.description @@@ 'premium'
ORDER BY p.id;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE products CASCADE;
DROP TABLE suppliers CASCADE;

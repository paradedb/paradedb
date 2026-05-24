-- Tests for SELECT DISTINCT pushdown into the BM25 index.
--
-- Plain SELECT DISTINCT (no DISTINCT ON) on a fast field maps to a
-- GROUP BY without aggregates. The AggregateScan hook now fires at
-- UPPERREL_DISTINCT in addition to UPPERREL_GROUP_AGG, routing through
-- Tantivy's TermsAggregation or DataFusion depending on cardinality.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- Setup
-- =============================================================================

DROP TABLE IF EXISTS dist_products CASCADE;

CREATE TABLE dist_products (
    id       SERIAL PRIMARY KEY,
    name     TEXT,
    category TEXT,
    brand    TEXT,
    rating   INTEGER,
    price    FLOAT,
    in_stock BOOLEAN
);

INSERT INTO dist_products (name, category, brand, rating, price, in_stock)
VALUES
    ('Laptop Pro',            'Electronics', 'BrandA', 5, 999.99,  true),
    ('Gaming Laptop',         'Electronics', 'BrandB', 4, 1299.99, true),
    ('Office Laptop',         'Electronics', 'BrandA', 3, 799.99,  false),
    ('Wireless Keyboard',     'Accessories', 'BrandC', 4, 79.99,   true),
    ('Mechanical Keyboard',   'Accessories', 'BrandA', 5, 149.99,  true),
    ('Running Shoes',         'Sports',      'BrandD', 5, 89.99,   true),
    ('Winter Jacket',         'Clothing',    'BrandE', 4, 129.99,  true),
    ('Summer Jacket',         'Clothing',    'BrandE', 3, 59.99,   true),
    ('Noise Headphones',      'Electronics', 'BrandB', 5, 299.99,  true),
    ('Budget Headphones',     'Electronics', 'BrandC', 3, 49.99,   true),
    ('Yoga Mat',              'Sports',      'BrandD', 4, 39.99,   true),
    ('Desk Chair',            'Furniture',   'BrandF', 5, 499.99,  true),
    ('Standing Desk',         'Furniture',   'BrandF', 4, 699.99,  false),
    ('USB Hub',               'Accessories', 'BrandC', 4, 29.99,   true),
    ('Monitor 4K',            'Electronics', 'BrandB', 5, 599.99,  true);

-- Index: category and brand are fast text fields; name is NOT fast (tests fallback);
-- rating and price are fast numeric fields.
CREATE INDEX dist_products_idx ON dist_products
USING bm25 (id, name, category, brand, rating, price)
WITH (
    key_field = 'id',
    text_fields  = '{"name": {}, "category": {"fast": true}, "brand": {"fast": true}}',
    numeric_fields = '{"rating": {"fast": true}, "price": {"fast": true}}'
);

ANALYZE dist_products;

-- =============================================================================
-- TEST 1: Basic DISTINCT on a fast text field
-- Expect: Custom Scan (ParadeDB Aggregate Scan) in the EXPLAIN plan.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR jacket OR shoes'
ORDER BY category;

SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR jacket OR shoes'
ORDER BY category;

-- Correctness: must match native PG (scan without custom scan)
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR jacket OR shoes'
ORDER BY category;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- TEST 2: DISTINCT on a fast numeric field
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT rating
FROM dist_products
WHERE name @@@ 'laptop OR keyboard'
ORDER BY rating;

SELECT DISTINCT rating
FROM dist_products
WHERE name @@@ 'laptop OR keyboard'
ORDER BY rating;

-- =============================================================================
-- TEST 3: DISTINCT with ORDER BY + LIMIT
-- Exercises the TopK / LIMIT-aware code path inside AggregateScan.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR jacket OR shoes OR headphones OR hub OR monitor OR mat OR chair OR desk'
ORDER BY category
LIMIT 3;

SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR jacket OR shoes OR headphones OR hub OR monitor OR mat OR chair OR desk'
ORDER BY category
LIMIT 3;

-- =============================================================================
-- TEST 4: Multi-column DISTINCT
-- DISTINCT on (category, brand) - both are fast fields.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT category, brand
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR headphones'
ORDER BY category, brand;

SELECT DISTINCT category, brand
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR headphones'
ORDER BY category, brand;

-- Correctness: compare with native PG
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT category, brand
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR headphones'
ORDER BY category, brand;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- TEST 5: DISTINCT on a non-fast field - must fall back gracefully
-- 'name' has no fast:true, so the custom scan cannot push down the
-- deduplication. It should fall back to native PG without an error.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT name
FROM dist_products
WHERE name @@@ 'laptop'
ORDER BY name;

SELECT DISTINCT name
FROM dist_products
WHERE name @@@ 'laptop'
ORDER BY name;

-- =============================================================================
-- TEST 6: DISTINCT ON - must fall back gracefully (DISTINCT ON is not supported)
-- DISTINCT ON uses a different key/sort pair which TermsAggregation cannot model.
-- The planner should decline and fall back to native PG execution.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT ON (category) category, name
FROM dist_products
WHERE name @@@ 'laptop OR keyboard'
ORDER BY category, name;

SELECT DISTINCT ON (category) category, name
FROM dist_products
WHERE name @@@ 'laptop OR keyboard'
ORDER BY category, name;

-- =============================================================================
-- TEST 7: DISTINCT with empty result set
-- WHERE clause matches nothing; DISTINCT must return zero rows.
-- =============================================================================

SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'nonexistent_xyz_term'
ORDER BY category;

-- =============================================================================
-- TEST 8: DISTINCT across entire table (no WHERE clause with @@@)
-- When there is no BM25 search predicate the hook should not fire and
-- native PG deduplication runs instead.
-- =============================================================================

SELECT DISTINCT category
FROM dist_products
ORDER BY category;

-- =============================================================================
-- Cleanup
-- =============================================================================

DROP TABLE IF EXISTS dist_products CASCADE;

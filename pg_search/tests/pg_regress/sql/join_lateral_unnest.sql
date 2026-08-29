-- =====================================================================
-- Tests for JoinScan and AggregateScan pushdown with LATERAL unnest
-- over multi-table joins.
--
-- Focuses on non-aggregate JoinScan queries with LATERAL unnest, with
-- one representative facet aggregate test.
--
-- Exercises:
-- 1. 2-table INNER JOIN + CROSS JOIN LATERAL unnest + LIMIT (JoinScan)
-- 2. 3-table JOIN + CROSS JOIN LATERAL unnest + LIMIT (JoinScan)
-- 3. LEFT JOIN LATERAL unnest (preserving NULL and empty array rows) (JoinScan)
-- 4. Multiple CROSS JOIN LATERAL unnests across different joined tables (JoinScan)
-- 5. Filtering on the unnested column in WHERE (JoinScan)
-- 6. Facet aggregation (COUNT DISTINCT) over unnested array on 3-table join (AggregateScan)
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO off;

-- =====================================================================
-- Test Data Setup
-- =====================================================================

CREATE TABLE jlu_products (
    id SERIAL PRIMARY KEY,
    title TEXT,
    categories TEXT[],
    price FLOAT
);

CREATE TABLE jlu_brands (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    brand_name TEXT,
    tags TEXT[]
);

CREATE TABLE jlu_stores (
    id SERIAL PRIMARY KEY,
    brand_id INTEGER,
    store_name TEXT,
    regions TEXT[]
);

INSERT INTO jlu_products (title, categories, price) VALUES
    ('Flagship Smartphone Pro', ARRAY['electronics', 'mobile'], 999.00),
    ('Noise Canceling Headphones', ARRAY['electronics', 'audio'], 299.00),
    ('Smart TV 4K OLED', ARRAY['electronics', 'video', 'home'], 1499.00),
    ('Ergonomic Office Chair', ARRAY['furniture', 'office'], 350.00),
    ('Mechanical Keyboard RGB', ARRAY['electronics', 'accessories'], 120.00),
    ('Mystery Item No Category', NULL, 50.00),
    ('Sample Item Empty Category', ARRAY[]::TEXT[], 25.00);

INSERT INTO jlu_brands (product_id, brand_name, tags) VALUES
    (1, 'TechCorp Global', ARRAY['premium', 'bestseller']),
    (2, 'SoundMaster Audio', ARRAY['audio_gear', 'wireless']),
    (3, 'VisionPlus Electronics', ARRAY['home_theater', 'bestseller']),
    (4, 'ErgoWorks Design', ARRAY['ergonomic']),
    (5, 'KeyPro Gaming', ARRAY['gaming', 'peripherals']),
    (6, 'GenericBrand', NULL),
    (7, 'ClearanceBrand', ARRAY[]::TEXT[]);

INSERT INTO jlu_stores (brand_id, store_name, regions) VALUES
    (1, 'TechCorp Flagship NYC', ARRAY['north_america', 'east_coast']),
    (2, 'SoundMaster London', ARRAY['europe', 'uk']),
    (3, 'VisionPlus Tokyo', ARRAY['asia', 'japan']),
    (4, 'ErgoWorks Berlin', ARRAY['europe', 'germany']),
    (5, 'KeyPro Online', ARRAY['global']);

-- =====================================================================
-- BM25 Index Creation with Array Fast Fields
-- =====================================================================

CREATE INDEX jlu_products_idx ON jlu_products
USING paradedb (id, title, (categories::pdb.literal), price)
WITH (key_field='id');

CREATE INDEX jlu_brands_idx ON jlu_brands
USING paradedb (id, product_id, brand_name, (tags::pdb.literal))
WITH (key_field='id');

CREATE INDEX jlu_stores_idx ON jlu_stores
USING paradedb (id, brand_id, store_name, (regions::pdb.literal))
WITH (key_field='id');

-- =====================================================================
-- TEST 1: Basic 2-table INNER JOIN with CROSS JOIN LATERAL unnest + LIMIT
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    p.title,
    b.brand_name,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title @@@ 'Smart' OR b.brand_name @@@ 'Electronics'
ORDER BY p.id, c
LIMIT 5;

SELECT
    p.id,
    p.title,
    b.brand_name,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title @@@ 'Smart' OR b.brand_name @@@ 'Electronics'
ORDER BY p.id, c
LIMIT 5;

-- =====================================================================
-- TEST 2: 3-table JOIN with CROSS JOIN LATERAL unnest on 2nd table + LIMIT
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    p.title,
    b.brand_name,
    s.store_name,
    t AS tag
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN jlu_stores s ON b.id = s.brand_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (
    p.title @@@ 'Smart'
    OR b.brand_name @@@ 'Electronics'
    OR s.store_name @@@ 'Electronics'
)
ORDER BY p.id, t
LIMIT 10;

SELECT
    p.id,
    p.title,
    b.brand_name,
    s.store_name,
    t AS tag
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN jlu_stores s ON b.id = s.brand_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (
    p.title @@@ 'Smart'
    OR b.brand_name @@@ 'Electronics'
    OR s.store_name @@@ 'Electronics'
)
ORDER BY p.id, t
LIMIT 10;

-- =====================================================================
-- TEST 3: LEFT JOIN LATERAL unnest (preserves NULL and empty array rows)
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    p.title,
    b.brand_name,
    t AS tag
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN LATERAL unnest(b.tags) AS t ON true
WHERE p.title @@@ 'Smart OR Mystery OR Empty' OR b.brand_name @@@ 'Electronics'
ORDER BY p.id, t NULLS LAST
LIMIT 10;

SELECT
    p.id,
    p.title,
    b.brand_name,
    t AS tag
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN LATERAL unnest(b.tags) AS t ON true
WHERE p.title @@@ 'Smart OR Mystery OR Empty' OR b.brand_name @@@ 'Electronics'
ORDER BY p.id, t NULLS LAST
LIMIT 10;

-- =====================================================================
-- TEST 4: Multiple CROSS JOIN LATERAL unnests across different tables
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    p.title,
    c AS category,
    r AS region
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
JOIN jlu_stores s ON b.id = s.brand_id
CROSS JOIN LATERAL unnest(p.categories) AS c
CROSS JOIN LATERAL unnest(s.regions) AS r
WHERE p.title @@@ 'Smart' OR b.brand_name @@@ 'Electronics'
ORDER BY p.id, c, r
LIMIT 10;

SELECT
    p.id,
    p.title,
    c AS category,
    r AS region
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
JOIN jlu_stores s ON b.id = s.brand_id
CROSS JOIN LATERAL unnest(p.categories) AS c
CROSS JOIN LATERAL unnest(s.regions) AS r
WHERE p.title @@@ 'Smart' OR b.brand_name @@@ 'Electronics'
ORDER BY p.id, c, r
LIMIT 10;

-- =====================================================================
-- TEST 5: Filtering on the unnested column in WHERE
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    p.title,
    b.brand_name,
    t AS tag
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (p.title @@@ 'Smart' OR b.brand_name @@@ 'Electronics')
  AND t = 'bestseller'
ORDER BY p.id, t
LIMIT 10;

SELECT
    p.id,
    p.title,
    b.brand_name,
    t AS tag
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (p.title @@@ 'Smart' OR b.brand_name @@@ 'Electronics')
  AND t = 'bestseller'
ORDER BY p.id, t
LIMIT 10;

-- =====================================================================
-- TEST 5b: Filtering on unnested column when column is omitted from SELECT
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    p.title,
    b.brand_name
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (p.title @@@ 'Smart' OR b.brand_name @@@ 'Electronics')
  AND t = 'bestseller'
ORDER BY p.id
LIMIT 10;

SELECT
    p.id,
    p.title,
    b.brand_name
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (p.title @@@ 'Smart' OR b.brand_name @@@ 'Electronics')
  AND t = 'bestseller'
ORDER BY p.id
LIMIT 10;

-- =====================================================================
-- TEST 6: Facet aggregation (COUNT DISTINCT) over unnest on 3-table join
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    t AS tag,
    count(DISTINCT p.id) AS product_count
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN jlu_stores s ON b.id = s.brand_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (
    p.title @@@ 'Smart'
    OR b.brand_name @@@ 'Electronics'
    OR s.store_name @@@ 'Electronics'
)
GROUP BY t
ORDER BY product_count DESC, t
LIMIT 10;

SELECT
    t AS tag,
    count(DISTINCT p.id) AS product_count
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN jlu_stores s ON b.id = s.brand_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (
    p.title @@@ 'Smart'
    OR b.brand_name @@@ 'Electronics'
    OR s.store_name @@@ 'Electronics'
)
GROUP BY t
ORDER BY product_count DESC, t
LIMIT 10;

-- =====================================================================
-- TEST 7: Truncated TopK pushdown strictly respecting unnested sort order
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title @@@ 'Smart'
ORDER BY p.id, c ASC
LIMIT 2;

SELECT
    p.id,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title @@@ 'Smart'
ORDER BY p.id, c ASC
LIMIT 2;

-- =====================================================================
-- TEST 8: ORDER BY on unnested column alone (DESC)
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title @@@ 'Smart'
ORDER BY c DESC
LIMIT 3;

SELECT
    p.id,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title @@@ 'Smart'
ORDER BY c DESC
LIMIT 3;

-- =====================================================================
-- TEST 9: CROSS JOIN LATERAL unnest drops empty arrays and NULLs
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    p.title,
    t AS tag
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (p.title @@@ 'Category' OR b.brand_name @@@ 'Brand')
ORDER BY p.id, t
LIMIT 10;

SELECT
    p.id,
    p.title,
    t AS tag
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (p.title @@@ 'Category' OR b.brand_name @@@ 'Brand')
ORDER BY p.id, t
LIMIT 10;

-- =====================================================================
-- Cleanup
-- =====================================================================
DROP TABLE jlu_stores CASCADE;
DROP TABLE jlu_brands CASCADE;
DROP TABLE jlu_products CASCADE;
RESET max_parallel_workers_per_gather;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_range_partitioned_join;


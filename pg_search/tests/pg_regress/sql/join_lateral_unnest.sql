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
USING paradedb (id, title, (categories::pdb.literal), price);

CREATE INDEX jlu_brands_idx ON jlu_brands
USING paradedb (id, product_id, brand_name, (tags::pdb.literal));

CREATE INDEX jlu_stores_idx ON jlu_stores
USING paradedb (id, brand_id, store_name, (regions::pdb.literal));

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
WHERE p.title ||| 'Smart' OR b.brand_name ||| 'Electronics'
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
WHERE p.title ||| 'Smart' OR b.brand_name ||| 'Electronics'
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
    p.title ||| 'Smart'
    OR b.brand_name ||| 'Electronics'
    OR s.store_name ||| 'Electronics'
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
    p.title ||| 'Smart'
    OR b.brand_name ||| 'Electronics'
    OR s.store_name ||| 'Electronics'
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
WHERE (p.title ||| 'Smart' OR p.title ||| 'Mystery' OR p.title ||| 'Empty') OR b.brand_name ||| 'Electronics'
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
WHERE (p.title ||| 'Smart' OR p.title ||| 'Mystery' OR p.title ||| 'Empty') OR b.brand_name ||| 'Electronics'
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
WHERE p.title ||| 'Smart' OR b.brand_name ||| 'Electronics'
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
WHERE p.title ||| 'Smart' OR b.brand_name ||| 'Electronics'
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
WHERE (p.title ||| 'Smart' OR b.brand_name ||| 'Electronics')
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
WHERE (p.title ||| 'Smart' OR b.brand_name ||| 'Electronics')
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
WHERE (p.title ||| 'Smart' OR b.brand_name ||| 'Electronics')
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
WHERE (p.title ||| 'Smart' OR b.brand_name ||| 'Electronics')
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
    p.title ||| 'Smart'
    OR b.brand_name ||| 'Electronics'
    OR s.store_name ||| 'Electronics'
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
    p.title ||| 'Smart'
    OR b.brand_name ||| 'Electronics'
    OR s.store_name ||| 'Electronics'
)
GROUP BY t
ORDER BY product_count DESC, t
LIMIT 10;

-- The unnested column alone as the sort key.
EXPLAIN (COSTS OFF)
SELECT
    t AS tag,
    count(DISTINCT p.id) AS product_count
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN jlu_stores s ON b.id = s.brand_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (
    p.title ||| 'Smart'
    OR b.brand_name ||| 'Electronics'
    OR s.store_name ||| 'Electronics'
)
GROUP BY t
ORDER BY t
LIMIT 10;

SELECT
    t AS tag,
    count(DISTINCT p.id) AS product_count
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN jlu_stores s ON b.id = s.brand_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (
    p.title ||| 'Smart'
    OR b.brand_name ||| 'Electronics'
    OR s.store_name ||| 'Electronics'
)
GROUP BY t
ORDER BY t
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
WHERE p.title ||| 'Smart'
ORDER BY p.id, c ASC
LIMIT 2;

SELECT
    p.id,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title ||| 'Smart'
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
WHERE p.title ||| 'Smart'
ORDER BY c DESC
LIMIT 3;

SELECT
    p.id,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title ||| 'Smart'
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
WHERE (p.title ||| 'Category' OR b.brand_name ||| 'Brand')
ORDER BY p.id, t
LIMIT 10;

SELECT
    p.id,
    p.title,
    t AS tag
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(b.tags) AS t
WHERE (p.title ||| 'Category' OR b.brand_name ||| 'Brand')
ORDER BY p.id, t
LIMIT 10;

-- =====================================================================
-- TEST 10: LEFT JOIN LATERAL unnest on a table pruned by an Anti Join
-- =====================================================================
INSERT INTO jlu_products (title, categories, price) VALUES
    ('Smart Solar Charger', ARRAY['electronics', 'solar'], 45.00);

EXPLAIN (COSTS OFF)
SELECT
    p.id,
    p.title,
    t AS tag
FROM jlu_products p
LEFT JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN LATERAL unnest(b.tags) AS t ON true
WHERE p.title ||| 'Smart' AND b.product_id IS NULL
ORDER BY p.id, t
LIMIT 5;

SELECT
    p.id,
    p.title,
    t AS tag
FROM jlu_products p
LEFT JOIN jlu_brands b ON p.id = b.product_id
LEFT JOIN LATERAL unnest(b.tags) AS t ON true
WHERE p.title ||| 'Smart' AND b.product_id IS NULL
ORDER BY p.id, t
LIMIT 5;

-- =====================================================================
-- TEST 10b: CROSS JOIN LATERAL unnest on a table pruned by an Anti Join
-- with chained LATERAL unnests and window aggregate (topk_as_agg)
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT
    p.id,
    p.title,
    COUNT(*) OVER (),
    t AS tag,
    c AS category,
    r AS region
FROM jlu_products p
LEFT JOIN jlu_brands b ON p.id = b.product_id
JOIN jlu_stores s ON p.id = s.brand_id
CROSS JOIN LATERAL unnest(b.tags) AS t
CROSS JOIN LATERAL unnest(p.categories) AS c
CROSS JOIN LATERAL unnest(s.regions) AS r
WHERE p.title ||| 'Smart' AND b.product_id IS NULL
ORDER BY p.id, s.id, t, c, r
LIMIT 5 OFFSET 1;

SELECT
    p.id,
    p.title,
    COUNT(*) OVER (),
    t AS tag,
    c AS category,
    r AS region
FROM jlu_products p
LEFT JOIN jlu_brands b ON p.id = b.product_id
JOIN jlu_stores s ON p.id = s.brand_id
CROSS JOIN LATERAL unnest(b.tags) AS t
CROSS JOIN LATERAL unnest(p.categories) AS c
CROSS JOIN LATERAL unnest(s.regions) AS r
WHERE p.title ||| 'Smart' AND b.product_id IS NULL
ORDER BY p.id, s.id, t, c, r
LIMIT 5 OFFSET 1;

-- =====================================================================
-- TEST 11: SELECT DISTINCT with CROSS JOIN LATERAL unnest + LIMIT
-- =====================================================================
EXPLAIN (COSTS OFF)
SELECT DISTINCT
    p.id,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title ||| 'Smart' OR b.brand_name ||| 'Electronics'
ORDER BY p.id, c
LIMIT 10;

SELECT DISTINCT
    p.id,
    c AS category
FROM jlu_products p
JOIN jlu_brands b ON p.id = b.product_id
CROSS JOIN LATERAL unnest(p.categories) AS c
WHERE p.title ||| 'Smart' OR b.brand_name ||| 'Electronics'
ORDER BY p.id, c
LIMIT 10;

-- =====================================================================
-- TEST 12: LATERAL jsonb_array_elements in JoinScan and AggregateScan (Issue #6694)
-- =====================================================================
SET paradedb.enable_custom_scan_without_operator = on;
SET paradedb.planner_warnings = 'off';

CREATE TABLE jlu_people (
    id SERIAL PRIMARY KEY,
    name TEXT,
    extra JSONB
);

CREATE TABLE jlu_mock_companies (
    id SERIAL PRIMARY KEY,
    domain TEXT,
    company_name TEXT
);

INSERT INTO jlu_people (name, extra) VALUES
    ('Alice', '{"current_employers": [{"title": "Software Engineer", "employer_company_website_domain": "google.com"}, {"title": "Consultant", "employer_company_website_domain": "acme.org"}]}'::jsonb),
    ('Bob', '{"current_employers": [{"title": "Data Scientist", "employer_company_website_domain": "meta.com"}]}'::jsonb),
    ('Charlie', '{"current_employers": [{"title": "Staff Engineer", "employer_company_website_domain": "google.com"}]}'::jsonb);

INSERT INTO jlu_mock_companies (domain, company_name) VALUES
    ('google.com', 'Google LLC'),
    ('meta.com', 'Meta Platforms'),
    ('acme.org', 'Acme Corporation');

CREATE INDEX jlu_people_idx ON jlu_people USING paradedb (id, name, (extra::pdb.unicode_words('columnar=true')));
CREATE INDEX jlu_mock_companies_idx ON jlu_mock_companies USING paradedb (id, (domain::pdb.literal), company_name);

-- AggregateScan: GROUP BY unnested JSON field (Issue #6694)
SELECT
    cur_emp->>'title' AS title,
    count(*) AS count
FROM jlu_people p
CROSS JOIN LATERAL jsonb_array_elements(p.extra->'current_employers') cur_emp
GROUP BY 1
ORDER BY count DESC, title;

-- JoinScan: 2-table join with LATERAL jsonb_array_elements_text
SELECT
    p.id,
    c.company_name,
    elem
FROM jlu_people p
JOIN jlu_mock_companies c ON p.id = c.id
CROSS JOIN LATERAL jsonb_array_elements_text(p.extra->'current_employers') elem
WHERE p.id = 2;

-- JoinScan: Filtering on unnested JSON field with LIKE
SELECT
    p.id,
    p.name,
    cur_emp->>'title' AS title
FROM jlu_people p
CROSS JOIN LATERAL jsonb_array_elements(p.extra->'current_employers') cur_emp
WHERE cur_emp->>'title' LIKE '%Engineer%'
ORDER BY p.id, title;

-- AggregateScan: Filtering on unnested JSON field with lower()
SELECT
    cur_emp->>'title' AS title,
    count(*) AS count
FROM jlu_people p
CROSS JOIN LATERAL jsonb_array_elements(p.extra->'current_employers') cur_emp
WHERE lower(cur_emp->>'title') = 'software engineer'
GROUP BY 1
ORDER BY count DESC, title;

-- Negative test: non-text array subscript (->0) declines gracefully to vanilla Postgres
SELECT p.id, cur_emp->0->>'title' AS first_title
FROM jlu_people p
CROSS JOIN LATERAL jsonb_array_elements(p.extra->'current_employers') cur_emp
ORDER BY p.id, first_title;

-- Negative test: jsonb operator (->) declines gracefully to vanilla Postgres
SELECT p.id, cur_emp->'title' AS json_title
FROM jlu_people p
CROSS JOIN LATERAL jsonb_array_elements(p.extra->'current_employers') cur_emp
ORDER BY p.id, json_title;

DROP TABLE jlu_mock_companies CASCADE;
DROP TABLE jlu_people CASCADE;
RESET paradedb.planner_warnings;
RESET paradedb.enable_custom_scan_without_operator;

-- =====================================================================
-- Cleanup
-- =====================================================================
DROP TABLE jlu_stores CASCADE;
DROP TABLE jlu_brands CASCADE;
DROP TABLE jlu_products CASCADE;
RESET max_parallel_workers_per_gather;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_range_partitioned_join;

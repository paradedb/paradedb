-- =====================================================================
-- Negative tests: verify graceful fallback to Postgres native plans
-- for unsupported aggregate-on-join query patterns.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test Data Setup (3 tables for multi-table join tests)
-- =====================================================================
CREATE TABLE fb_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT
);

CREATE TABLE fb_tags (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    tag_name TEXT
);

CREATE TABLE fb_reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    rating INTEGER
);

INSERT INTO fb_products (description, category, price) VALUES
    ('Laptop computer fast', 'Electronics', 999.99),
    ('Running shoes light', 'Sports', 89.99),
    ('Winter jacket warm', 'Clothing', 129.99);

INSERT INTO fb_tags (product_id, tag_name) VALUES
    (1, 'tech'), (2, 'fitness'), (3, 'outdoor');

INSERT INTO fb_reviews (product_id, rating) VALUES
    (1, 5), (1, 4), (2, 3), (3, 4);

CREATE INDEX fb_products_idx ON fb_products
USING bm25 (id, description, category, price)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}}'
);

CREATE INDEX fb_tags_idx ON fb_tags
USING bm25 (id, product_id, tag_name)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}}',
    text_fields='{"tag_name": {"fast": true}}'
);

CREATE INDEX fb_reviews_idx ON fb_reviews
USING bm25 (id, product_id, rating)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}, "rating": {"fast": true}}'
);

-- =====================================================================
-- Test 1: 3-table join → should use DataFusion backend
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop';

SELECT COUNT(*)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop';

-- Test 1b: 3-table join with GROUP BY
SELECT p.category, COUNT(*), SUM(r.rating)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- Test 1c: 3-table join parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(r.rating)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 1d: 3-table join with multiple aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*), SUM(r.rating), AVG(r.rating), MIN(r.rating), MAX(r.rating)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category;

SELECT p.category, COUNT(*), SUM(r.rating), AVG(r.rating), MIN(r.rating), MAX(r.rating)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- Test 1e: 3-table star schema (tags and reviews both join to products)
SELECT COUNT(*)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop';

-- Test 1f: 3-table chain join (products → tags → reviews via tag)
-- Note: fb_reviews doesn't have tag_id, so we join on product_id for both
-- This tests the equi-key injection at the correct join level
SELECT t.tag_name, COUNT(*), SUM(r.rating)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY t.tag_name
ORDER BY t.tag_name;

-- Test 1g: 3-table with LEFT JOIN → should use DataFusion backend
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*), COUNT(r.rating)
FROM fb_products p
LEFT JOIN fb_tags t ON p.id = t.product_id
LEFT JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category;

SELECT p.category, COUNT(*), COUNT(r.rating)
FROM fb_products p
LEFT JOIN fb_tags t ON p.id = t.product_id
LEFT JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
-- Test 2: CROSS JOIN → should fall back to Postgres native
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*)
FROM fb_products p
CROSS JOIN fb_tags t
WHERE p.description @@@ 'laptop';

SELECT COUNT(*)
FROM fb_products p
CROSS JOIN fb_tags t
WHERE p.description @@@ 'laptop';

-- =====================================================================
-- Test 3: HAVING clause → should now use DataFusion
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 0;

SELECT p.category, COUNT(*)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 0;

-- Test 3b: HAVING parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 0
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT p.category, COUNT(*)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 0
ORDER BY p.category;

-- =====================================================================
-- Test 4: HAVING with aggregate NOT in SELECT list (hidden aggregate)
-- The HAVING clause's COUNT(*) is not in the SELECT list — it should
-- be computed as a hidden aggregate by DataFusion, not cause fallback.
-- =====================================================================

-- Test 4a: SELECT SUM, HAVING COUNT > 1
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, SUM(r.rating)
FROM fb_products p
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 1;

SELECT p.category, SUM(r.rating)
FROM fb_products p
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 1
ORDER BY p.category;

-- Test 4b: Parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, SUM(r.rating)
FROM fb_products p
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 1
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 4c: HAVING with threshold that filters everything
SELECT p.category, AVG(r.rating)
FROM fb_products p
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 100
ORDER BY p.category;

-- Test 4d: Multiple hidden aggregates — HAVING with AND
SELECT p.category, MIN(r.rating)
FROM fb_products p
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 1 AND SUM(r.rating) > 5
ORDER BY p.category;

-- Test 4d parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, MIN(r.rating)
FROM fb_products p
JOIN fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 1 AND SUM(r.rating) > 5
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Clean up
-- =====================================================================
DROP TABLE fb_reviews;
DROP TABLE fb_tags;
DROP TABLE fb_products;

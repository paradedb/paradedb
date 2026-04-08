-- =====================================================================
-- Edge case tests for aggregate-on-join via DataFusion backend
-- =====================================================================
-- Tests: non-unique join keys, 3+ table outer joins, mixed join types

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test Data Setup
-- =====================================================================
CREATE TABLE ec_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT
);

CREATE TABLE ec_reviews (
    id SERIAL PRIMARY KEY,
    category TEXT,
    rating INTEGER,
    reviewer TEXT
);

CREATE TABLE ec_suppliers (
    id SERIAL PRIMARY KEY,
    category TEXT,
    supplier_name TEXT
);

INSERT INTO ec_products (description, category, price) VALUES
    ('Laptop computer', 'Electronics', 999.99),
    ('Desktop monitor', 'Electronics', 499.99),
    ('Running shoes', 'Sports', 89.99),
    ('Tennis racket', 'Sports', 149.99),
    ('Winter jacket', 'Clothing', 129.99);

INSERT INTO ec_reviews (category, rating, reviewer) VALUES
    ('Electronics', 5, 'alice'),
    ('Electronics', 4, 'bob'),
    ('Electronics', 3, 'cloe'),
    ('Sports', 4, 'alice'),
    ('Sports', 5, 'bob'),
    ('Clothing', 3, 'cloe'),
    ('Clothing', 4, 'alice');

INSERT INTO ec_suppliers (category, supplier_name) VALUES
    ('Electronics', 'TechCorp'),
    ('Electronics', 'ChipMakers'),
    ('Sports', 'AthletePro'),
    ('Clothing', 'FashionInc'),
    ('Clothing', 'StyleHouse');

CREATE INDEX ec_products_idx ON ec_products
USING bm25 (id, description, category, price)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}}'
);

CREATE INDEX ec_reviews_idx ON ec_reviews
USING bm25 (id, category, rating, reviewer)
WITH (
    key_field='id',
    text_fields='{"category": {"fast": true}, "reviewer": {"fast": true}}',
    numeric_fields='{"rating": {"fast": true}}'
);

CREATE INDEX ec_suppliers_idx ON ec_suppliers
USING bm25 (id, category, supplier_name)
WITH (
    key_field='id',
    text_fields='{"category": {"fast": true}, "supplier_name": {"fast": true}}'
);

-- =====================================================================
-- Test 1: Non-unique join key — JOIN on category (many-to-many)
-- Expected: fan-out produces more rows than either table alone
-- =====================================================================

-- Test 1a: COUNT(*) with non-unique key (category)
-- 2 Electronics products × 3 Electronics reviews = 6
-- 2 Sports products × 2 Sports reviews = 4
-- 1 Clothing product × 2 Clothing reviews = 2
-- Total: 12 matched rows
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*)
FROM ec_products p
JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket';

SELECT COUNT(*)
FROM ec_products p
JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket';

-- Test 1b: GROUP BY with non-unique key
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*), SUM(r.rating), AVG(r.rating)
FROM ec_products p
JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), SUM(r.rating), AVG(r.rating)
FROM ec_products p
JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket'
GROUP BY p.category
ORDER BY p.category;

-- Test 1c: Parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(r.rating), AVG(r.rating)
FROM ec_products p
JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 1d: Non-unique key with MIN/MAX
SELECT p.category, MIN(r.rating), MAX(r.rating), MIN(p.price), MAX(p.price)
FROM ec_products p
JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket'
GROUP BY p.category
ORDER BY p.category;

-- Test 1e: LEFT JOIN with non-unique key — all products, even unmatched
-- If we add a product with category 'Books' (no reviews), LEFT JOIN should include it
INSERT INTO ec_products (description, category, price) VALUES
    ('Science fiction novel', 'Books', 19.99);
-- Re-index is automatic on INSERT

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*), COUNT(r.rating)
FROM ec_products p
LEFT JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket OR novel'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), COUNT(r.rating)
FROM ec_products p
LEFT JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket OR novel'
GROUP BY p.category
ORDER BY p.category;

-- Parity for LEFT JOIN non-unique
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), COUNT(r.rating)
FROM ec_products p
LEFT JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket OR novel'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 2: 3-table join with non-unique keys
-- products × reviews × suppliers all joined on category
-- =====================================================================

-- Test 2a: 3-table INNER join on non-unique category
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*), SUM(r.rating)
FROM ec_products p
JOIN ec_reviews r ON p.category = r.category
JOIN ec_suppliers s ON p.category = s.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), SUM(r.rating)
FROM ec_products p
JOIN ec_reviews r ON p.category = r.category
JOIN ec_suppliers s ON p.category = s.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket'
GROUP BY p.category
ORDER BY p.category;

-- Test 2b: Parity — DataFusion vs Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(r.rating)
FROM ec_products p
JOIN ec_reviews r ON p.category = r.category
JOIN ec_suppliers s ON p.category = s.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 3: 3-table LEFT JOIN — should use DataFusion backend
-- =====================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*), COUNT(r.rating), COUNT(s.supplier_name)
FROM ec_products p
LEFT JOIN ec_reviews r ON p.category = r.category
LEFT JOIN ec_suppliers s ON p.category = s.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket OR novel'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), COUNT(r.rating), COUNT(s.supplier_name)
FROM ec_products p
LEFT JOIN ec_reviews r ON p.category = r.category
LEFT JOIN ec_suppliers s ON p.category = s.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket OR novel'
GROUP BY p.category
ORDER BY p.category;

-- Parity for 3-table LEFT JOIN
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), COUNT(r.rating), COUNT(s.supplier_name)
FROM ec_products p
LEFT JOIN ec_reviews r ON p.category = r.category
LEFT JOIN ec_suppliers s ON p.category = s.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket OR novel'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 4: FULL OUTER JOIN with non-unique keys (2-table)
-- =====================================================================

SELECT p.category, COUNT(*), COUNT(r.rating)
FROM ec_products p
FULL JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket OR novel'
GROUP BY p.category
ORDER BY p.category;

-- Parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), COUNT(r.rating)
FROM ec_products p
FULL JOIN ec_reviews r ON p.category = r.category
WHERE p.description @@@ 'laptop OR shoes OR jacket OR monitor OR racket OR novel'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Clean up
-- =====================================================================
DROP TABLE ec_suppliers;
DROP TABLE ec_reviews;
DROP TABLE ec_products;

-- =====================================================================
-- Multi-table (3+) aggregate-on-join via DataFusion Backend
-- =====================================================================
-- Tests aggregate functions on 3-table and 4-table joins executed via
-- the DataFusion custom scan backend.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test Data Setup (4 tables)
-- =====================================================================
CREATE TABLE mt_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT,
    in_stock BOOLEAN
);

CREATE TABLE mt_tags (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    tag_name TEXT
);

CREATE TABLE mt_reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    rating INTEGER
);

CREATE TABLE mt_suppliers (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    supplier_name TEXT
);

INSERT INTO mt_products (description, category, price, in_stock) VALUES
    ('Laptop fast processor', 'Electronics', 999.99, true),
    ('Gaming laptop RGB', 'Electronics', 1299.99, true),
    ('Running shoes athlete', 'Sports', 89.99, true),
    ('Winter jacket warm', 'Clothing', 129.99, false),
    ('Toy laptop kids', 'Toys', 49.99, true);

INSERT INTO mt_tags (product_id, tag_name) VALUES
    (1, 'tech'), (1, 'computer'),
    (2, 'tech'), (2, 'gaming'),
    (3, 'fitness'), (3, 'running'),
    (4, 'outdoor'),
    (5, 'tech'), (5, 'kids');

INSERT INTO mt_reviews (product_id, rating) VALUES
    (1, 5), (1, 4), (2, 3), (3, 4), (4, 3);

INSERT INTO mt_suppliers (product_id, supplier_name) VALUES
    (1, 'TechCorp'), (2, 'GameInc'), (3, 'SportCo'), (4, 'WearIt');

CREATE INDEX mt_products_idx ON mt_products
USING bm25 (id, description, category, price, in_stock)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}}',
    boolean_fields='{"in_stock": {"fast": true}}'
);

CREATE INDEX mt_tags_idx ON mt_tags
USING bm25 (id, product_id, tag_name)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}}',
    text_fields='{"tag_name": {"fast": true}}'
);

CREATE INDEX mt_reviews_idx ON mt_reviews
USING bm25 (id, product_id, rating)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}, "rating": {"fast": true}}'
);

CREATE INDEX mt_suppliers_idx ON mt_suppliers
USING bm25 (id, product_id, supplier_name)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}}',
    text_fields='{"supplier_name": {"fast": true}}'
);

-- =====================================================================
-- Section 1: 3-table INNER JOIN with COUNT/SUM/AVG
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*), SUM(r.rating), AVG(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category;

SELECT p.category, COUNT(*), SUM(r.rating), AVG(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- Parity check against native Postgres
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(r.rating), AVG(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Section 2: 4-table INNER JOIN
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*), SUM(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
JOIN mt_suppliers s ON p.id = s.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category;

SELECT p.category, COUNT(*), SUM(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
JOIN mt_suppliers s ON p.id = s.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- Parity check
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
JOIN mt_suppliers s ON p.id = s.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Section 3: 3-table scalar aggregate (no GROUP BY)
-- =====================================================================
SELECT COUNT(*), SUM(r.rating), MIN(r.rating), MAX(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop';

-- =====================================================================
-- Section 4: 3-table with mixed join types (INNER + LEFT)
-- =====================================================================
SELECT p.category, COUNT(*), COUNT(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
LEFT JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR kids'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
-- Section 5: 3-table GROUP BY columns from different tables
-- =====================================================================
SELECT p.category, t.tag_name, COUNT(*), SUM(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop'
GROUP BY p.category, t.tag_name
ORDER BY p.category, t.tag_name;

-- =====================================================================
-- Section 6: 3-table HAVING clause
-- =====================================================================
SELECT p.category, COUNT(*), SUM(r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 2
ORDER BY p.category;

-- =====================================================================
-- Section 7: 3-table TopK (ORDER BY aggregate + LIMIT)
-- =====================================================================
SELECT p.category, COUNT(*) AS cnt, SUM(r.rating) AS total
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY cnt DESC
LIMIT 2;

-- =====================================================================
-- Section 8: SUM(DISTINCT) on 3-table join
-- =====================================================================
SELECT p.category, SUM(DISTINCT r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- Parity check
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, SUM(DISTINCT r.rating)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Section 9: STRING_AGG on 3-table join
-- =====================================================================
SELECT p.category, STRING_AGG(DISTINCT t.tag_name, ', ')
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
-- Section 10: BOOL_AND/BOOL_OR on 3-table join
-- =====================================================================
SELECT p.category, BOOL_AND(p.in_stock), BOOL_OR(p.in_stock)
FROM mt_products p
JOIN mt_tags t ON p.id = t.product_id
JOIN mt_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
-- Clean up
-- =====================================================================
DROP TABLE mt_suppliers;
DROP TABLE mt_reviews;
DROP TABLE mt_tags;
DROP TABLE mt_products;

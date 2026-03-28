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
-- Test 1: 3-table join → should fall back to Postgres native
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
-- Test 3: HAVING clause → should fall back to Postgres native
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 1;

SELECT p.category, COUNT(*)
FROM fb_products p
JOIN fb_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
HAVING COUNT(*) > 1;

-- =====================================================================
-- Clean up
-- =====================================================================
DROP TABLE fb_reviews;
DROP TABLE fb_tags;
DROP TABLE fb_products;

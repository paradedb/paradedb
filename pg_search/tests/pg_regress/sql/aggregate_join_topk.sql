-- =====================================================================
-- TopK Aggregate-on-JOIN via DataFusion Backend
-- =====================================================================
-- Tests ORDER BY aggregate + LIMIT pushdown into DataFusion for join
-- aggregate queries, using the TopKAggregateRule optimization.
-- Also tests GROUP BY on joins (requires custom_scan_tlist for scanrelid=0).

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test Data Setup
-- =====================================================================
CREATE TABLE topk_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT,
    rating INTEGER
);

CREATE TABLE topk_tags (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    tag_name TEXT
);

INSERT INTO topk_products (description, category, price, rating) VALUES
    ('Laptop with fast processor', 'Electronics', 999.99, 5),
    ('Gaming laptop with RGB', 'Electronics', 1299.99, 5),
    ('Wireless mouse for office', 'Electronics', 29.99, 4),
    ('Running shoes for athletes', 'Sports', 89.99, 4),
    ('Basketball shoes premium', 'Sports', 119.99, 3),
    ('Winter jacket warm', 'Clothing', 129.99, 3),
    ('Summer dress casual', 'Clothing', 49.99, 4),
    ('Toy laptop for kids', 'Toys', 499.99, 2),
    ('Puzzle game educational', 'Toys', 19.99, 5),
    ('Cookbook healthy recipes', 'Books', 24.99, 4);

INSERT INTO topk_tags (product_id, tag_name) VALUES
    (1, 'tech'), (1, 'computer'),
    (2, 'tech'), (2, 'gaming'),
    (3, 'tech'), (3, 'office'),
    (4, 'fitness'), (4, 'running'),
    (5, 'fitness'), (5, 'basketball'),
    (6, 'outdoor'),
    (7, 'fashion'),
    (8, 'tech'), (8, 'kids'),
    (9, 'kids'), (9, 'education'),
    (10, 'cooking');

CREATE INDEX topk_products_idx ON topk_products
USING bm25 (id, description, category, price, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}, "rating": {"fast": true}}'
);

CREATE INDEX topk_tags_idx ON topk_tags
USING bm25 (id, product_id, tag_name)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}}',
    text_fields='{"tag_name": {"fast": true}}'
);

-- =====================================================================
-- Test 1: GROUP BY on join (requires custom_scan_tlist fix)
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category;

SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category;

-- =====================================================================
-- Test 2: ORDER BY COUNT(*) DESC LIMIT — TopK pushdown
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 3;

SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 3;

-- =====================================================================
-- Test 3: ORDER BY SUM(price) DESC LIMIT
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, SUM(p.price)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY SUM(p.price) DESC
LIMIT 2;

SELECT p.category, SUM(p.price)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY SUM(p.price) DESC
LIMIT 2;

-- =====================================================================
-- Test 4: ORDER BY COUNT(*) ASC LIMIT (bottom K)
-- =====================================================================
SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) ASC
LIMIT 2;

-- =====================================================================
-- Test 5: Multiple aggregates with ORDER BY one of them
-- =====================================================================
SELECT p.category, COUNT(*), SUM(p.price), MIN(p.rating), MAX(p.rating)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY SUM(p.price) DESC
LIMIT 3;

-- =====================================================================
-- Test 6: Parity — TopK results match full ORDER BY
-- =====================================================================
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(p.price)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT p.category, COUNT(*), SUM(p.price)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 3;

-- =====================================================================
-- Test 7: Scalar aggregates (no GROUP BY) still work
-- =====================================================================
SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- =====================================================================
-- Test 8: LIMIT 1 (smallest possible K)
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 1;

SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 1;

-- =====================================================================
-- Test 9: LIMIT larger than number of groups (returns all groups)
-- =====================================================================
SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 100;

-- =====================================================================
-- Test 10: OFFSET + LIMIT on join TopK
-- =====================================================================
SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 2 OFFSET 1;

-- =====================================================================
-- Test 11: Parity — TopK top-3 matches top-3 of full result
-- =====================================================================
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 3;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT p.category, COUNT(*)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 3;

-- =====================================================================
-- Test 12: ORDER BY SUM ASC LIMIT (bottom K by sum)
-- =====================================================================
SELECT p.category, SUM(p.price)
FROM topk_products p
JOIN topk_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR dress OR toy OR puzzle OR cookbook'
GROUP BY p.category
ORDER BY SUM(p.price) ASC
LIMIT 2;

DROP TABLE topk_tags;
DROP TABLE topk_products;

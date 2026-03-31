-- Test aggregate-on-join execution via DataFusion backend
SET paradedb.enable_aggregate_custom_scan TO on;

-- Create two tables with BM25 indexes
CREATE TABLE agg_join_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT,
    rating INTEGER
);

CREATE TABLE agg_join_tags (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    tag_name TEXT
);

INSERT INTO agg_join_products (description, category, price, rating) VALUES
    ('Laptop with fast processor', 'Electronics', 999.99, 5),
    ('Gaming laptop with RGB', 'Electronics', 1299.99, 5),
    ('Running shoes for athletes', 'Sports', 89.99, 4),
    ('Winter jacket warm', 'Clothing', 129.99, 3);

INSERT INTO agg_join_tags (product_id, tag_name) VALUES
    (1, 'tech'), (1, 'computer'),
    (2, 'tech'), (2, 'gaming'),
    (3, 'fitness'), (3, 'running'),
    (4, 'outdoor');

CREATE INDEX agg_join_products_idx ON agg_join_products
USING bm25 (id, description, category, price, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}, "rating": {"fast": true}}'
);

CREATE INDEX agg_join_tags_idx ON agg_join_tags
USING bm25 (id, product_id, tag_name)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}}',
    text_fields='{"tag_name": {"fast": true}}'
);

-- Test 1: Scalar COUNT(*) on join
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 2: Multiple scalar aggregates on join
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 3: MIN/MAX on join
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT MIN(p.price), MAX(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT MIN(p.price), MAX(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 4: Empty result set
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent_term_xyz';

SELECT COUNT(*)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent_term_xyz';

-- Test 5: GROUP BY + ORDER BY on join
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), SUM(p.price)
FROM agg_join_products p
JOIN agg_join_tags t ON p.id = t.product_id
GROUP BY p.category
ORDER BY p.category;

-- Test 6: Verify single-table aggregates still use Tantivy backend
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM agg_join_products WHERE description @@@ 'laptop';

SELECT COUNT(*) FROM agg_join_products WHERE description @@@ 'laptop';

-- Clean up
DROP TABLE agg_join_tags;
DROP TABLE agg_join_products;

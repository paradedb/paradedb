-- =====================================================================
-- Parallel Aggregate-on-JOIN via DataFusion Two-Phase Aggregation
-- =====================================================================
-- Tests DataFusion aggregate execution with configurable target
-- partitions and memory pool integration.
-- When target_partitions > 1, DataFusion produces two-phase aggregate
-- plans (Partial → Repartition → FinalPartitioned) that execute
-- cooperatively on the backend thread.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test Data Setup
-- =====================================================================
CREATE TABLE par_agg_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT,
    rating INTEGER
);

CREATE TABLE par_agg_tags (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    tag_name TEXT
);

INSERT INTO par_agg_products (description, category, price, rating) VALUES
    ('Laptop with fast processor', 'Electronics', 999.99, 5),
    ('Gaming laptop with RGB', 'Electronics', 1299.99, 5),
    ('Running shoes for athletes', 'Sports', 89.99, 4),
    ('Winter jacket warm', 'Clothing', 129.99, 3),
    ('Toy laptop for kids', 'Toys', 499.99, 2);

INSERT INTO par_agg_tags (product_id, tag_name) VALUES
    (1, 'tech'), (1, 'computer'),
    (2, 'tech'), (2, 'gaming'),
    (3, 'fitness'), (3, 'running'),
    (4, 'outdoor'),
    (5, 'tech'), (5, 'kids');

CREATE INDEX par_agg_products_idx ON par_agg_products
USING bm25 (id, description, category, price, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}, "rating": {"fast": true}}'
);

CREATE INDEX par_agg_tags_idx ON par_agg_tags
USING bm25 (id, product_id, tag_name)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}}',
    text_fields='{"tag_name": {"fast": true}}'
);

-- =====================================================================
-- SECTION 1: Default (target_partitions=1) with memory pool
-- Verifies that the memory pool integration doesn't break execution.
-- =====================================================================

-- Test 1.1: Scalar COUNT(*)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT COUNT(*)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 1.2: Multiple scalar aggregates
SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 1.3: MIN/MAX
SELECT MIN(p.price), MAX(p.price)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 1.4: GROUP BY with COUNT
SELECT p.category, COUNT(*)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

-- Test 1.5: GROUP BY with multiple aggregates
SELECT p.category, COUNT(*), SUM(p.price), MIN(p.rating), MAX(p.rating)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category;

-- Test 1.6: Empty result
SELECT COUNT(*)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent_term_xyz';

SELECT SUM(p.price), AVG(p.rating)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent_term_xyz';

-- =====================================================================
-- SECTION 2: Two-phase scalar aggregates (target_partitions > 1)
-- Scalar aggregates (no GROUP BY) work correctly with multi-partition
-- plans. The two-phase structure (Partial → Final) is used.
-- =====================================================================
SET paradedb.aggregate_target_partitions TO 4;

-- Test 2.1: Scalar COUNT(*) with parallel partitions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT COUNT(*)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 2.2: Multiple scalar aggregates with parallel partitions
SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 2.3: MIN/MAX with parallel partitions
SELECT MIN(p.price), MAX(p.price)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 2.4: Empty result with parallel partitions
SELECT COUNT(*)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent_term_xyz';

-- Test 2.5: Broader query scalar aggregates
SELECT COUNT(*), SUM(p.price), AVG(p.price), MIN(p.rating), MAX(p.rating)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy';

-- =====================================================================
-- SECTION 3: Parity checks
-- =====================================================================

-- Test 3.1: Parity — target_partitions=1 vs Postgres native (no custom scan)
SET paradedb.aggregate_target_partitions TO 1;
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT COUNT(*), SUM(p.price), AVG(p.rating), MIN(p.price), MAX(p.price)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT COUNT(*), SUM(p.price), AVG(p.rating), MIN(p.price), MAX(p.price)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 3.2: Parity — target_partitions=4 scalar vs target_partitions=1
SET paradedb.aggregate_target_partitions TO 4;
SELECT COUNT(*), SUM(p.price), AVG(p.rating), MIN(p.price), MAX(p.price)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 3.3: Parity — GROUP BY results: Postgres native vs DataFusion
SET paradedb.aggregate_target_partitions TO 1;
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT p.category, COUNT(*), SUM(p.price)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT p.category, COUNT(*), SUM(p.price)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
<<<<<<< HEAD
=======
-- SECTION 4: TopK with memory pool (target_partitions=1)
-- =====================================================================

-- Test 4.1: ORDER BY COUNT(*) DESC LIMIT
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT p.category, COUNT(*)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 2;

SELECT p.category, COUNT(*)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy'
GROUP BY p.category
ORDER BY COUNT(*) DESC
LIMIT 2;

-- Test 4.2: ORDER BY SUM DESC LIMIT
SELECT p.category, SUM(p.price)
FROM par_agg_products p
JOIN par_agg_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR toy'
GROUP BY p.category
ORDER BY SUM(p.price) DESC
LIMIT 2;

-- =====================================================================
>>>>>>> 3fd0a5474 (feat: add configurable target partitions and memory pool for DataFusion aggregates)
-- Clean up
-- =====================================================================
DROP TABLE par_agg_tags;
DROP TABLE par_agg_products;

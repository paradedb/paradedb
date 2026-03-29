-- =====================================================================
-- Single-table DataFusion fallback when Tantivy bucket limits exceeded
-- =====================================================================
-- When estimated_groups > max_term_agg_buckets, AggregateScan routes
-- single-table (BASEREL) aggregates through the DataFusion backend
-- instead of Tantivy. This test verifies that path works correctly.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test Data Setup — enough rows + distinct values for reliable estimates
-- =====================================================================
CREATE TABLE df_fallback_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT,
    rating INTEGER
);

INSERT INTO df_fallback_products (description, category, price, rating) VALUES
    ('Laptop computer fast', 'Electronics', 999.99, 5),
    ('Gaming laptop RGB', 'Electronics', 1299.99, 4),
    ('Running shoes light', 'Sports', 89.99, 4),
    ('Winter jacket warm', 'Clothing', 129.99, 3),
    ('Toy robot fun', 'Toys', 49.99, 2),
    ('Coffee maker brew', 'Kitchen', 79.99, 5),
    ('Headphones wireless', 'Audio', 199.99, 4),
    ('Yoga mat stretch', 'Fitness', 29.99, 3),
    ('Book novel read', 'Books', 14.99, 5),
    ('Pen ballpoint write', 'Office', 2.99, 3),
    ('Desk wooden sit', 'Furniture', 399.99, 4),
    ('Lamp bright light', 'Lighting', 59.99, 4);

CREATE INDEX df_fallback_products_idx ON df_fallback_products
USING bm25 (id, description, category, price, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}, "rating": {"fast": true}}'
);

-- ANALYZE so Postgres gets accurate group count estimates
ANALYZE df_fallback_products;

-- =====================================================================
-- SECTION 1: Verify Tantivy is used with default bucket limit
-- =====================================================================

-- Test 1.1: With default bucket limit, single-table uses Tantivy (shows Index:)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category;

SELECT category, COUNT(*)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category
ORDER BY category;

-- =====================================================================
-- SECTION 2: Force DataFusion fallback with low bucket limit
-- =====================================================================

-- Set bucket limit to 1 to guarantee DataFusion fallback
-- (any GROUP BY with > 1 group triggers the fallback)
SET paradedb.max_term_agg_buckets TO 1;

-- Test 2.1: EXPLAIN should show Backend: DataFusion (not Index:)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category;

-- Test 2.2: Results should be correct (all 11 groups, not truncated)
SELECT category, COUNT(*)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category
ORDER BY category;

-- Test 2.3: Multiple aggregates via DataFusion fallback
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*), SUM(price), AVG(rating), MIN(price), MAX(price)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category;

SELECT category, COUNT(*), SUM(price), AVG(rating), MIN(price), MAX(price)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category
ORDER BY category;

-- Test 2.4: Scalar aggregate (no GROUP BY) via DataFusion fallback
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*), SUM(price)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes';

SELECT COUNT(*), SUM(price)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes';

-- =====================================================================
-- SECTION 3: Parity — DataFusion fallback vs Postgres native
-- =====================================================================

-- Test 3.1: Compare DataFusion fallback with Postgres native
-- DataFusion fallback (bucket limit still 1)
SELECT category, COUNT(*), SUM(price)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category
ORDER BY category;

-- Postgres native
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT category, COUNT(*), SUM(price)
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category
ORDER BY category;

-- Restore settings
SET paradedb.enable_aggregate_custom_scan TO on;
RESET paradedb.max_term_agg_buckets;

-- =====================================================================
-- Clean up
-- =====================================================================
DROP TABLE df_fallback_products;

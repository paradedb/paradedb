-- =====================================================================
-- Single-table DataFusion fallback when Tantivy bucket limits exceeded
-- =====================================================================
-- When estimated_groups > max_term_agg_buckets, AggregateScan routes
-- single-table (BASEREL) aggregates through the DataFusion backend
-- instead of Tantivy. This test verifies that path works correctly.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
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
    ('Lamp bright light', 'Lighting', 59.99, 4),
    ('Garden hose green', 'Garden', 19.99, NULL),
    ('Garden gloves thick', 'Garden', 9.99, NULL);

-- One row per group makes the functionally-dependent output case explicit;
-- the large text expression exercises per-tuple projection allocation.
INSERT INTO df_fallback_products (description, category, price, rating)
SELECT 'Memory projection row ' || g, 'Memory', g, 1
FROM generate_series(1, 2048) g;

CREATE INDEX df_fallback_products_idx ON df_fallback_products
USING paradedb (id, description, category, price, rating)
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

-- Test 2.2: With bucket limit = 1, the estimate exceeds the cap so it routes to
-- DataFusion and returns every group (no truncation).
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

-- Test 2.4: PostgreSQL evaluates wrappers and multi-aggregate expressions
-- over the flat DataFusion tuple.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category, COUNT(*), COUNT(*) * 2, COUNT(*)::numeric, COUNT(*) / 2.0
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY 2 DESC
LIMIT 2;

SELECT category, COUNT(*), COUNT(*) * 2, COUNT(*)::numeric, COUNT(*) / 2.0
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY 2 DESC
LIMIT 2;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT SUM(rating) + COUNT(*) AS combined,
       SUM(rating)::text || ':' || category AS labeled
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

SELECT SUM(rating) + COUNT(*) AS combined,
       SUM(rating)::text || ':' || category AS labeled
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

-- Test 2.5: NULL, casts and COALESCE remain PostgreSQL expressions.
SELECT category, COUNT(rating), MAX(rating)::text, COALESCE(SUM(rating), 0),
       COUNT(rating) * 2, AVG(rating)::numeric(4, 2)
FROM df_fallback_products
WHERE description @@@ 'garden OR laptop'
GROUP BY category
ORDER BY 2 DESC;

-- Test 2.6: HAVING identifies filtered aggregates by complete Aggref identity.
-- The output and HAVING contain the same two COUNT(*) calls, but the HAVING
-- predicate must refer to the second filter rather than the first one.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT category,
       COUNT(*) FILTER (WHERE rating >= 4) + COUNT(*) FILTER (WHERE rating <= 3) AS rated
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category
HAVING COUNT(*) FILTER (WHERE rating <= 3) > 0
ORDER BY category;

SELECT category,
       COUNT(*) FILTER (WHERE rating >= 4) + COUNT(*) FILTER (WHERE rating <= 3) AS rated
FROM df_fallback_products
WHERE description @@@ 'laptop OR shoes OR jacket OR robot OR coffee OR headphones OR yoga OR book OR pen OR desk OR lamp'
GROUP BY category
HAVING COUNT(*) FILTER (WHERE rating <= 3) > 0
ORDER BY category;

-- Test 2.7: category is functionally dependent on the grouped primary key.
-- It must be a real raw grouping input, never an uninitialised resjunk slot.
-- The text group key also exercises by-reference Arrow-to-Datum conversion.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id, category, upper(category),
       repeat(category, 1024) || ':' || COUNT(*)::text AS label
FROM df_fallback_products
WHERE description @@@ 'memory'
GROUP BY id;

SELECT COUNT(*) AS groups,
       BOOL_AND(category = 'Memory') AS categories_ok,
       BOOL_AND(upper_category = 'MEMORY') AS wrappers_ok,
       MIN(octet_length(label)) AS min_label_bytes,
       MAX(octet_length(label)) AS max_label_bytes
FROM (
    SELECT id, category, upper(category) AS upper_category,
           repeat(category, 1024) || ':' || COUNT(*)::text AS label
    FROM df_fallback_products
    WHERE description @@@ 'memory'
    GROUP BY id
) projected_groups;

-- Test 2.8: DISTINCT with an ORDER BY whose sort-group order differs from the
-- output column order.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT DISTINCT category, id
FROM df_fallback_products
WHERE description @@@ 'memory'
ORDER BY id DESC;

SELECT COUNT(*) AS groups,
       MIN(category) AS only_category,
       MAX(id) - MIN(id) + 1 AS id_span
FROM (
    SELECT DISTINCT category, id
    FROM df_fallback_products
    WHERE description @@@ 'memory'
    ORDER BY id DESC
) distinct_groups;

-- Test 2.9: Scalar aggregate (no GROUP BY) stays on Tantivy — it produces a
-- single row and cannot truncate, so the bucket cap is irrelevant.
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

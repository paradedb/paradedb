-- A GROUP BY on a deferred string column groups on term ordinals first, decodes one
-- row per group, and merges the segments' groups by string.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 0;

-- The NUMERIC price routes the aggregates to the DataFusion backend.
CREATE TABLE aog_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    brand TEXT,
    price NUMERIC(8, 2)
);
CREATE TABLE aog_reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    stars INTEGER
);
CREATE TABLE aog_notes (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    note TEXT
);

-- Three insert batches, each its own segment, so the same category groups per
-- segment before the merge. A few categories and brands repeat across many rows,
-- which is the case grouping on ordinals is for.
CREATE INDEX aog_products_idx ON aog_products
USING bm25 (id, description, category, brand, price)
WITH (key_field='id',
      text_fields='{"description": {}, "category": {"fast": true}, "brand": {"fast": true}}',
      numeric_fields='{"price": {"fast": true}}');
CREATE INDEX aog_reviews_idx ON aog_reviews
USING bm25 (id, product_id, stars)
WITH (key_field='id', numeric_fields='{"product_id": {"fast": true}, "stars": {"fast": true}}');
CREATE INDEX aog_notes_idx ON aog_notes
USING bm25 (id, product_id, note)
WITH (key_field='id', numeric_fields='{"product_id": {"fast": true}}', text_fields='{"note": {"fast": true}}');

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO aog_products (description, category, brand, price)
SELECT CASE WHEN i % 10 = 0 THEN 'gadget ' ELSE 'widget ' END || i,
       (ARRAY['Tools', 'Toys', 'Garden', NULL])[1 + i % 4],
       (ARRAY['Acme', 'Bolt', 'Cog'])[1 + i % 3],
       (i % 9) + 1
FROM generate_series(1, 60) AS i;
INSERT INTO aog_products (description, category, brand, price)
SELECT CASE WHEN i % 10 = 0 THEN 'gadget ' ELSE 'widget ' END || i,
       (ARRAY['Tools', 'Toys', 'Garden', NULL])[1 + i % 4],
       (ARRAY['Acme', 'Bolt', 'Cog'])[1 + i % 3],
       (i % 9) + 1
FROM generate_series(61, 120) AS i;
INSERT INTO aog_products (description, category, brand, price)
SELECT CASE WHEN i % 10 = 0 THEN 'gadget ' ELSE 'widget ' END || i,
       (ARRAY['Tools', 'Toys', 'Garden', NULL])[1 + i % 4],
       (ARRAY['Acme', 'Bolt', 'Cog'])[1 + i % 3],
       (i % 9) + 1
FROM generate_series(121, 180) AS i;
RESET paradedb.global_mutable_segment_rows;

INSERT INTO aog_reviews (product_id, stars)
SELECT id, s FROM aog_products, generate_series(1, 3) AS s WHERE id % 3 <> 0;
INSERT INTO aog_notes (product_id, note)
SELECT id, (ARRAY['fragile', 'heavy'])[1 + id % 2] FROM aog_products WHERE id % 5 = 1;

-- =============================================================================
-- One table: the fetch runs below the partial aggregate, the decode above it
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category
ORDER BY category;

SELECT category, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category
ORDER BY category;

-- The same groups from Postgres.
SET paradedb.enable_aggregate_custom_scan TO off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category
ORDER BY category;

SELECT category, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category
ORDER BY category;

SET paradedb.enable_aggregate_custom_scan TO on;

-- Two string keys group together on their ordinal pairs.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, brand, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category, brand
ORDER BY category, brand;

SELECT category, brand, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category, brand
ORDER BY category, brand;

-- DISTINCT is a group by with no aggregates.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT category
FROM aog_products
WHERE description @@@ 'widget'
ORDER BY category;

SELECT DISTINCT category
FROM aog_products
WHERE description @@@ 'widget'
ORDER BY category;

-- =============================================================================
-- A fan-out join: the scan resolves the ordinals, the partial aggregate groups
-- the joined rows on them, and one row per group is decoded
-- =============================================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.category, COUNT(*), SUM(r.stars)
FROM aog_products p JOIN aog_reviews r ON r.product_id = p.id
WHERE p.description @@@ 'widget'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), SUM(r.stars)
FROM aog_products p JOIN aog_reviews r ON r.product_id = p.id
WHERE p.description @@@ 'widget'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.category, COUNT(*), SUM(r.stars)
FROM aog_products p JOIN aog_reviews r ON r.product_id = p.id
WHERE p.description @@@ 'widget'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), SUM(r.stars)
FROM aog_products p JOIN aog_reviews r ON r.product_id = p.id
WHERE p.description @@@ 'widget'
GROUP BY p.category
ORDER BY p.category;

SET paradedb.enable_aggregate_custom_scan TO on;

-- A string from the null-supplying side of an outer join stays deferred through
-- the join: a null-extended row is a NULL ordinal, which counts as no note.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.category, COUNT(*), COUNT(n.note)
FROM aog_products p LEFT JOIN aog_notes n ON n.product_id = p.id
WHERE p.description @@@ 'widget'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), COUNT(n.note)
FROM aog_products p LEFT JOIN aog_notes n ON n.product_id = p.id
WHERE p.description @@@ 'widget'
GROUP BY p.category
ORDER BY p.category;

-- =============================================================================
-- Opting out: with the decode pinned to the scan, the aggregate sees strings
-- =============================================================================

SET paradedb.defer_string_decode = off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category
ORDER BY category;

SELECT category, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category
ORDER BY category;

RESET paradedb.defer_string_decode;

-- =============================================================================
-- MPP: each worker groups its segments on ordinals and decodes its groups
-- =============================================================================

SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category
ORDER BY category;

SELECT category, COUNT(*), SUM(price)
FROM aog_products
WHERE description @@@ 'widget'
GROUP BY category
ORDER BY category;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.category, COUNT(*), SUM(r.stars)
FROM aog_products p JOIN aog_reviews r ON r.product_id = p.id
WHERE p.description @@@ 'widget'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), SUM(r.stars)
FROM aog_products p JOIN aog_reviews r ON r.product_id = p.id
WHERE p.description @@@ 'widget'
GROUP BY p.category
ORDER BY p.category;

DROP TABLE aog_notes;
DROP TABLE aog_reviews;
DROP TABLE aog_products;

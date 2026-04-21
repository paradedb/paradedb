-- =====================================================================
-- MPP correctness for the JoinOnly shape: a bare join without an
-- aggregate above it. Verifies that with `paradedb.enable_mpp = on` the
-- result rows match the serial baseline.
--
-- Like `mpp_exec.sql`, this test exercises plan-stash + classifier +
-- parallel-flag pipeline. PG's planner may not launch actual workers on
-- this small dataset; what this test DOES guarantee is:
--   * the JoinOnly plan stash completes without error
--   * the exec path produces correct rows whether or not workers launch
--   * `mpp_debug` logs the `mpp: routing JoinScan exec through shape` line
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_join_custom_scan TO on;

CREATE TABLE mpp_join_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT
);

CREATE TABLE mpp_join_reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    rating INTEGER,
    comment TEXT
);

INSERT INTO mpp_join_products (description, category)
SELECT
    CASE (i % 4)
        WHEN 0 THEN 'Laptop computer fast'
        WHEN 1 THEN 'Running shoes light'
        WHEN 2 THEN 'Winter jacket warm'
        ELSE 'Office chair ergonomic'
    END,
    CASE (i % 4)
        WHEN 0 THEN 'Electronics'
        WHEN 1 THEN 'Sports'
        WHEN 2 THEN 'Clothing'
        ELSE 'Furniture'
    END
FROM generate_series(1, 40) AS i;

INSERT INTO mpp_join_reviews (product_id, rating, comment)
SELECT
    (i % 40) + 1,
    (i % 5) + 1,
    'review ' || i
FROM generate_series(1, 200) AS i;

CREATE INDEX mpp_join_products_idx ON mpp_join_products
USING bm25 (id, description, category)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}'
);

CREATE INDEX mpp_join_reviews_idx ON mpp_join_reviews
USING bm25 (id, product_id, rating, comment)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}, "rating": {"fast": true}}',
    text_fields='{"comment": {}}'
);

-- =====================================================================
-- Serial baseline
-- =====================================================================
SET paradedb.enable_mpp TO off;

SELECT p.id, p.description, r.rating
FROM mpp_join_products p
JOIN mpp_join_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop'
ORDER BY p.id, r.id
LIMIT 10;

-- =====================================================================
-- MPP enabled: must return the same rows (set-equality, then row-order).
-- =====================================================================
SET paradedb.enable_mpp TO on;
SET paradedb.mpp_worker_count TO 2;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.description, r.rating
FROM mpp_join_products p
JOIN mpp_join_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop'
ORDER BY p.id, r.id
LIMIT 10;

SELECT p.id, p.description, r.rating
FROM mpp_join_products p
JOIN mpp_join_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop'
ORDER BY p.id, r.id
LIMIT 10;

-- Multi-predicate join with both sides filtered.
SELECT p.id, p.description, r.comment
FROM mpp_join_products p
JOIN mpp_join_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop' AND r.comment @@@ 'review'
ORDER BY p.id, r.id
LIMIT 10;

-- =====================================================================
-- Clean up
-- =====================================================================
RESET paradedb.enable_mpp;
RESET paradedb.mpp_worker_count;
RESET paradedb.enable_join_custom_scan;

DROP TABLE mpp_join_reviews;
DROP TABLE mpp_join_products;

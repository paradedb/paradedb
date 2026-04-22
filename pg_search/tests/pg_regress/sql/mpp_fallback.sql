-- =====================================================================
-- MPP fallback: prove that `paradedb.enable_mpp = on` is a no-op for
-- queries whose shape isn't (yet) wired to the MPP path.
--
-- Phase 4b-i landed the GUC + all the primitives + the customscan_glue
-- helpers, but did NOT yet flip AggregateScan's or JoinScan's
-- `ParallelQueryCapable` hooks. So with `enable_mpp=on` every query
-- should still produce the exact same answers as with `enable_mpp=off`
-- (there just isn't an MPP path to choose). This test locks in that
-- no-regression invariant — if Phase 4b-ii accidentally changes the
-- off-path result under `enable_mpp=on`, this diff catches it.
--
-- Data setup mirrors the non-MPP aggregate fallback test so the two are
-- easy to compare.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

CREATE TABLE mpp_fb_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT
);

CREATE TABLE mpp_fb_reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    rating INTEGER
);

INSERT INTO mpp_fb_products (description, category, price) VALUES
    ('Laptop computer fast', 'Electronics', 999.99),
    ('Running shoes light', 'Sports', 89.99),
    ('Winter jacket warm', 'Clothing', 129.99),
    ('Office chair ergonomic', 'Furniture', 249.99);

INSERT INTO mpp_fb_reviews (product_id, rating) VALUES
    (1, 5), (1, 4), (2, 3), (3, 4), (3, 5), (4, 2);

CREATE INDEX mpp_fb_products_idx ON mpp_fb_products
USING bm25 (id, description, category, price)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"price": {"fast": true}}'
);

CREATE INDEX mpp_fb_reviews_idx ON mpp_fb_reviews
USING bm25 (id, product_id, rating)
WITH (
    key_field='id',
    numeric_fields='{"product_id": {"fast": true}, "rating": {"fast": true}}'
);

-- =====================================================================
-- Baseline: enable_mpp = off (current default behavior)
-- =====================================================================
SET paradedb.enable_mpp TO off;

-- Scalar aggregate on join
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*)
FROM mpp_fb_products p
JOIN mpp_fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR chair';

SELECT COUNT(*)
FROM mpp_fb_products p
JOIN mpp_fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR chair';

-- GROUP BY aggregate on join
SELECT p.category, COUNT(*), MAX(r.rating) AS max_rating
FROM mpp_fb_products p
JOIN mpp_fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR chair'
GROUP BY p.category
ORDER BY p.category;

-- =====================================================================
-- MPP enabled: results must be byte-identical.
--
-- With MPP on but no customscan actually routes to the MPP path yet,
-- the queries must return the same answers as with MPP off. Any
-- divergence here means Phase 4b-ii accidentally changed existing
-- behavior on the enable_mpp=on branch.
-- =====================================================================
SET paradedb.enable_mpp TO on;
SET paradedb.mpp_worker_count TO 2;

-- Same EXPLAIN + query as above.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*)
FROM mpp_fb_products p
JOIN mpp_fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR chair';

SELECT COUNT(*)
FROM mpp_fb_products p
JOIN mpp_fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR chair';

SELECT p.category, COUNT(*), MAX(r.rating) AS max_rating
FROM mpp_fb_products p
JOIN mpp_fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR chair'
GROUP BY p.category
ORDER BY p.category;

-- Also verify `mpp_debug = on` is a no-op for correctness (may emit
-- diagnostic warnings, but the query results must be unchanged).
SET paradedb.mpp_debug TO on;
SELECT COUNT(*)
FROM mpp_fb_products p
JOIN mpp_fb_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR chair';
SET paradedb.mpp_debug TO off;

-- Restore defaults.
RESET paradedb.enable_mpp;
RESET paradedb.mpp_worker_count;
RESET paradedb.enable_aggregate_custom_scan;

DROP TABLE mpp_fb_reviews;
DROP TABLE mpp_fb_products;

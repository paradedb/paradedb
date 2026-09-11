-- =============================================================================
-- Non-Equi Joins for AggregateScan (DataFusion Backend)
-- =============================================================================
-- Tests aggregate functions (COUNT, SUM, AVG, MIN, MAX, GROUP BY, HAVING) on
-- non-equi and mixed equi/non-equi JOIN queries executed via DataFusion.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS agg_nonequi_products CASCADE;
DROP TABLE IF EXISTS agg_nonequi_promos CASCADE;
DROP TABLE IF EXISTS agg_nonequi_tiers CASCADE;

CREATE TABLE agg_nonequi_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT,
    rating INTEGER
);

CREATE TABLE agg_nonequi_promos (
    id SERIAL PRIMARY KEY,
    description TEXT,
    promo_code TEXT,
    min_price FLOAT,
    max_price FLOAT,
    min_rating INTEGER
);

CREATE TABLE agg_nonequi_tiers (
    id SERIAL PRIMARY KEY,
    description TEXT,
    tier_name TEXT,
    threshold_price FLOAT
);

INSERT INTO agg_nonequi_products (description, category, price, rating) VALUES
    ('Gaming laptop fast RGB', 'electronics', 1299.99, 5),
    ('Work laptop lightweight', 'electronics', 899.99, 4),
    ('Mechanical keyboard clicky', 'electronics', 89.99, 5),
    ('Running shoes breathable', 'sports', 79.99, 4),
    ('Trail running shoes durable', 'sports', 119.99, 3),
    ('Winter jacket insulated', 'clothing', 149.99, 4),
    ('Rain jacket waterproof', 'clothing', 69.99, 2);

INSERT INTO agg_nonequi_promos (description, promo_code, min_price, max_price, min_rating) VALUES
    ('Summer tech discounts', 'electronics', 50.00, 200.00, 4),
    ('Premium electronics gala', 'electronics', 500.00, 2000.00, 4),
    ('Outdoor gear event', 'sports', 50.00, 150.00, 3),
    ('All store sitewide savings', 'all', 0.00, 100.00, 1);

INSERT INTO agg_nonequi_tiers (description, tier_name, threshold_price) VALUES
    ('Tier 1 budget threshold', 'bronze', 50.00),
    ('Tier 2 standard threshold', 'silver', 100.00),
    ('Tier 3 luxury threshold', 'gold', 500.00);

CREATE INDEX agg_nonequi_products_idx ON agg_nonequi_products
USING paradedb (id, description, category, price, rating)
WITH (
    key_field = 'id',
    text_fields = '{"description": {}, "category": {"fast": true}}',
    numeric_fields = '{"price": {"fast": true}, "rating": {"fast": true}}'
);

CREATE INDEX agg_nonequi_promos_idx ON agg_nonequi_promos
USING paradedb (id, description, promo_code, min_price, max_price, min_rating)
WITH (
    key_field = 'id',
    text_fields = '{"description": {}, "promo_code": {"fast": true}}',
    numeric_fields = '{"min_price": {"fast": true}, "max_price": {"fast": true}, "min_rating": {"fast": true}}'
);

CREATE INDEX agg_nonequi_tiers_idx ON agg_nonequi_tiers
USING paradedb (id, description, tier_name, threshold_price)
WITH (
    key_field = 'id',
    text_fields = '{"description": {}, "tier_name": {"fast": true}}',
    numeric_fields = '{"threshold_price": {"fast": true}}'
);

-- =============================================================================
-- SECTION 1: Scalar Aggregates over Non-Equi INNER JOINs (no GROUP BY)
-- =============================================================================

-- Test 1.1: COUNT(*), SUM, AVG over range join condition (price BETWEEN min_price AND max_price)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price >= pr.min_price AND p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR jacket';

SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price >= pr.min_price AND p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR jacket';

-- Test 1.2: MIN, MAX over pure non-equi inequality (rating >= min_rating)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT MIN(p.price), MAX(p.price), COUNT(*)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.rating >= pr.min_rating
WHERE pr.description @@@ 'tech OR gala';

SELECT MIN(p.price), MAX(p.price), COUNT(*)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.rating >= pr.min_rating
WHERE pr.description @@@ 'tech OR gala';

-- Test 1.3: Mixed equi and non-equi join condition (category = promo_code AND price <= max_price)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*), SUM(p.price), MIN(p.rating), MAX(p.rating)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.category = pr.promo_code AND p.price <= pr.max_price
WHERE p.description @@@ 'keyboard OR laptop';

SELECT COUNT(*), SUM(p.price), MIN(p.rating), MAX(p.rating)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.category = pr.promo_code AND p.price <= pr.max_price
WHERE p.description @@@ 'keyboard OR laptop';

-- =============================================================================
-- SECTION 2: Aggregates with GROUP BY and HAVING
-- =============================================================================

-- Test 2.1: GROUP BY promo_code over non-equi join
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pr.promo_code, COUNT(*), SUM(p.price)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price >= pr.min_price AND p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR keyboard'
GROUP BY pr.promo_code
ORDER BY pr.promo_code;

SELECT pr.promo_code, COUNT(*), SUM(p.price)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price >= pr.min_price AND p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR keyboard'
GROUP BY pr.promo_code
ORDER BY pr.promo_code;

-- Test 2.2: GROUP BY category with HAVING filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*), AVG(p.price)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR jacket OR shoes'
GROUP BY p.category
HAVING COUNT(*) > 1
ORDER BY p.category;

SELECT p.category, COUNT(*), AVG(p.price)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR jacket OR shoes'
GROUP BY p.category
HAVING COUNT(*) > 1
ORDER BY p.category;

-- =============================================================================
-- SECTION 3: Non-Equi OUTER JOIN Aggregates
-- =============================================================================

-- Test 3.1: Scalar LEFT JOIN aggregate (ensuring unmatched rows count toward COUNT(*) but null in COUNT(pr.id))
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*), COUNT(pr.id), SUM(p.price)
FROM agg_nonequi_products p
LEFT JOIN agg_nonequi_promos pr ON p.category = pr.promo_code AND p.price > pr.max_price
WHERE p.description @@@ 'laptop OR jacket OR keyboard';

SELECT COUNT(*), COUNT(pr.id), SUM(p.price)
FROM agg_nonequi_products p
LEFT JOIN agg_nonequi_promos pr ON p.category = pr.promo_code AND p.price > pr.max_price
WHERE p.description @@@ 'laptop OR jacket OR keyboard';

-- Test 3.2: LEFT JOIN with GROUP BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*), COUNT(pr.id)
FROM agg_nonequi_products p
LEFT JOIN agg_nonequi_promos pr ON p.price > pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*), COUNT(pr.id)
FROM agg_nonequi_products p
LEFT JOIN agg_nonequi_promos pr ON p.price > pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- =============================================================================
-- SECTION 4: 3-Table Non-Equi JOIN Aggregates
-- =============================================================================

-- Test 4.1: 3 tables joined via mixed equi and non-equi conditions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT t.tier_name, COUNT(*), SUM(p.price)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.category = pr.promo_code AND p.price <= pr.max_price
JOIN agg_nonequi_tiers t ON p.price >= t.threshold_price
WHERE p.description @@@ 'laptop OR keyboard'
GROUP BY t.tier_name
ORDER BY t.tier_name;

SELECT t.tier_name, COUNT(*), SUM(p.price)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.category = pr.promo_code AND p.price <= pr.max_price
JOIN agg_nonequi_tiers t ON p.price >= t.threshold_price
WHERE p.description @@@ 'laptop OR keyboard'
GROUP BY t.tier_name
ORDER BY t.tier_name;

-- =============================================================================
-- SECTION 5: Parity Checks (Aggregate Custom Scan ON vs OFF)
-- =============================================================================

-- Test 5.1: Scalar aggregate parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price >= pr.min_price AND p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR jacket';

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT COUNT(*), SUM(p.price), AVG(p.rating)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price >= pr.min_price AND p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR jacket';

-- Test 5.2: GROUP BY aggregate parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT pr.promo_code, COUNT(*), SUM(p.price)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price >= pr.min_price AND p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR keyboard'
GROUP BY pr.promo_code
ORDER BY pr.promo_code;

SET paradedb.enable_aggregate_custom_scan TO on;
SELECT pr.promo_code, COUNT(*), SUM(p.price)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.price >= pr.min_price AND p.price <= pr.max_price
WHERE p.description @@@ 'laptop OR shoes OR keyboard'
GROUP BY pr.promo_code
ORDER BY pr.promo_code;

-- =============================================================================
-- SECTION 6: Fallback / Rejection
-- =============================================================================

-- Add an unindexed column
ALTER TABLE agg_nonequi_products ADD COLUMN unindexed_stock INT;
UPDATE agg_nonequi_products SET unindexed_stock = 10;

-- Test 6.1: Non-equi join on unindexed column should gracefully fall back
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.unindexed_stock < pr.min_rating
WHERE p.description @@@ 'laptop';

SELECT COUNT(*)
FROM agg_nonequi_products p
JOIN agg_nonequi_promos pr ON p.unindexed_stock < pr.min_rating
WHERE p.description @@@ 'laptop';

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS agg_nonequi_products CASCADE;
DROP TABLE IF EXISTS agg_nonequi_promos CASCADE;
DROP TABLE IF EXISTS agg_nonequi_tiers CASCADE;

RESET max_parallel_workers_per_gather;
RESET paradedb.enable_aggregate_custom_scan;

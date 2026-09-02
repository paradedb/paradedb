-- =============================================================================
-- Non-Equi Joins for JoinScan Custom Scan
-- =============================================================================
-- Tests non-equi join conditions (<, <=, >, >=, <>, BETWEEN), mixed equi and
-- non-equi conditions, outer joins, and fallback behavior in JoinScan.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS nonequi_items CASCADE;
DROP TABLE IF EXISTS nonequi_offers CASCADE;

CREATE TABLE nonequi_items (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT,
    price DECIMAL(10,2),
    rating INTEGER
);

CREATE TABLE nonequi_offers (
    id INTEGER PRIMARY KEY,
    promo_name TEXT,
    description TEXT,
    target_category TEXT,
    min_price DECIMAL(10,2),
    max_price DECIMAL(10,2),
    min_rating INTEGER
);

INSERT INTO nonequi_items (id, name, description, category, price, rating) VALUES
(101, 'Mechanical Keyboard', 'Ergonomic clicky mechanical keyboard for typing', 'electronics', 89.99, 5),
(102, 'Wireless Mouse', 'Ergonomic wireless optical mouse with bluetooth', 'electronics', 29.99, 4),
(103, 'USB-C Cable', 'Durable high speed charging and data cable', 'accessories', 9.99, 3),
(104, 'Gaming Headset', 'Surround sound gaming headset with noise canceling', 'electronics', 149.99, 5),
(105, 'Laptop Stand', 'Adjustable aluminum laptop riser for desk', 'accessories', 39.99, 4),
(106, 'Desk Mat', 'Extra large waterproof desk mat mousepad', 'accessories', 19.99, 2);

INSERT INTO nonequi_offers (id, promo_name, description, target_category, min_price, max_price, min_rating) VALUES
(201, 'Budget Saver', 'Special discount on budget accessories and electronics', 'accessories', 5.00, 35.00, 3),
(202, 'Midrange Deal', 'Great value for mid-tier productivity devices', 'electronics', 25.00, 100.00, 4),
(203, 'Premium Tech', 'Exclusive promo on high-end electronics', 'electronics', 80.00, 200.00, 5),
(204, 'All Clear', 'Clearance discounts across all categories', 'all', 0.00, 50.00, 1);

-- All join, filter, and order-by columns must be fast fields for JoinScan
CREATE INDEX nonequi_items_idx ON nonequi_items USING paradedb (id, name, description, category, price, rating)
WITH (
    key_field = 'id',
    text_fields = '{"name": {"fast": true}, "category": {"fast": true}}',
    numeric_fields = '{"price": {"fast": true}, "rating": {"fast": true}}'
);

CREATE INDEX nonequi_offers_idx ON nonequi_offers USING paradedb (id, promo_name, description, target_category, min_price, max_price, min_rating)
WITH (
    key_field = 'id',
    text_fields = '{"promo_name": {"fast": true}, "target_category": {"fast": true}}',
    numeric_fields = '{"min_price": {"fast": true}, "max_price": {"fast": true}, "min_rating": {"fast": true}}'
);

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- SECTION 1: Pure Non-Equi INNER JOINs
-- =============================================================================

-- Test 1.1: Less-than-or-equal condition (price <= max_price)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name, i.price, o.max_price
FROM nonequi_items i
JOIN nonequi_offers o ON i.price <= o.max_price
WHERE i.description @@@ 'ergonomic'
ORDER BY i.id, o.id
LIMIT 10;

SELECT i.id, i.name, o.promo_name, i.price, o.max_price
FROM nonequi_items i
JOIN nonequi_offers o ON i.price <= o.max_price
WHERE i.description @@@ 'ergonomic'
ORDER BY i.id, o.id
LIMIT 10;

-- Test 1.2: Greater-than-or-equal condition on offers table search predicate
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name, i.rating, o.min_rating
FROM nonequi_items i
JOIN nonequi_offers o ON i.rating >= o.min_rating
WHERE o.description @@@ 'exclusive OR value'
ORDER BY i.id, o.id
LIMIT 10;

SELECT i.id, i.name, o.promo_name, i.rating, o.min_rating
FROM nonequi_items i
JOIN nonequi_offers o ON i.rating >= o.min_rating
WHERE o.description @@@ 'exclusive OR value'
ORDER BY i.id, o.id
LIMIT 10;

-- Test 1.3: Range condition (price BETWEEN min_price AND max_price)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name, i.price, o.min_price, o.max_price
FROM nonequi_items i
JOIN nonequi_offers o ON i.price >= o.min_price AND i.price <= o.max_price
WHERE i.description @@@ 'keyboard OR mouse OR headset'
ORDER BY i.id, o.id
LIMIT 10;

SELECT i.id, i.name, o.promo_name, i.price, o.min_price, o.max_price
FROM nonequi_items i
JOIN nonequi_offers o ON i.price >= o.min_price AND i.price <= o.max_price
WHERE i.description @@@ 'keyboard OR mouse OR headset'
ORDER BY i.id, o.id
LIMIT 10;

-- Test 1.4: Inequality condition (<>) with search predicate on both sides
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name, i.category, o.target_category
FROM nonequi_items i
JOIN nonequi_offers o ON i.category <> o.target_category
WHERE i.description @@@ 'wireless' AND o.description @@@ 'budget'
ORDER BY i.id, o.id
LIMIT 10;

SELECT i.id, i.name, o.promo_name, i.category, o.target_category
FROM nonequi_items i
JOIN nonequi_offers o ON i.category <> o.target_category
WHERE i.description @@@ 'wireless' AND o.description @@@ 'budget'
ORDER BY i.id, o.id
LIMIT 10;

-- =============================================================================
-- SECTION 2: Mixed Equi and Non-Equi INNER JOINs
-- =============================================================================

-- Test 2.1: Category equality + price range non-equality
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name, i.price, o.max_price
FROM nonequi_items i
JOIN nonequi_offers o ON i.category = o.target_category AND i.price <= o.max_price
WHERE i.description @@@ 'cable OR riser OR mat'
ORDER BY i.id, o.id
LIMIT 10;

SELECT i.id, i.name, o.promo_name, i.price, o.max_price
FROM nonequi_items i
JOIN nonequi_offers o ON i.category = o.target_category AND i.price <= o.max_price
WHERE i.description @@@ 'cable OR riser OR mat'
ORDER BY i.id, o.id
LIMIT 10;

-- Test 2.2: Category equality + multiple non-equi conditions (price and rating)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name, i.price, i.rating
FROM nonequi_items i
JOIN nonequi_offers o ON i.category = o.target_category
                     AND i.price >= o.min_price
                     AND i.rating >= o.min_rating
WHERE i.description @@@ 'electronics OR keyboard OR headset'
ORDER BY i.id, o.id
LIMIT 10;

SELECT i.id, i.name, o.promo_name, i.price, i.rating
FROM nonequi_items i
JOIN nonequi_offers o ON i.category = o.target_category
                     AND i.price >= o.min_price
                     AND i.rating >= o.min_rating
WHERE i.description @@@ 'electronics OR keyboard OR headset'
ORDER BY i.id, o.id
LIMIT 10;

-- =============================================================================
-- SECTION 3: Non-Equi OUTER JOINs
-- =============================================================================

-- Test 3.1: LEFT JOIN with pure non-equi ON condition
-- Preserved left rows with no qualifying offer must be null-extended
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, i.price, o.promo_name, o.max_price
FROM nonequi_items i
LEFT JOIN nonequi_offers o ON i.price > o.max_price AND o.min_rating >= 4
WHERE i.description @@@ 'keyboard OR cable OR headset'
ORDER BY i.id, o.id NULLS LAST
LIMIT 10;

SELECT i.id, i.name, i.price, o.promo_name, o.max_price
FROM nonequi_items i
LEFT JOIN nonequi_offers o ON i.price > o.max_price AND o.min_rating >= 4
WHERE i.description @@@ 'keyboard OR cable OR headset'
ORDER BY i.id, o.id NULLS LAST
LIMIT 10;

-- Test 3.2: LEFT JOIN with mixed equi + non-equi ON condition
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name, i.price, o.min_price
FROM nonequi_items i
LEFT JOIN nonequi_offers o ON i.category = o.target_category AND i.price < o.min_price
WHERE i.description @@@ 'cable OR mat OR mouse'
ORDER BY i.id, o.id NULLS LAST
LIMIT 10;

SELECT i.id, i.name, o.promo_name, i.price, o.min_price
FROM nonequi_items i
LEFT JOIN nonequi_offers o ON i.category = o.target_category AND i.price < o.min_price
WHERE i.description @@@ 'cable OR mat OR mouse'
ORDER BY i.id, o.id NULLS LAST
LIMIT 10;

-- Test 3.3: RIGHT JOIN with non-equi ON condition
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name, o.max_price
FROM nonequi_items i
RIGHT JOIN nonequi_offers o ON i.category = o.target_category AND i.price <= o.max_price
WHERE o.description @@@ 'exclusive OR budget'
ORDER BY o.id, i.id NULLS LAST
LIMIT 10;

SELECT i.id, i.name, o.promo_name, o.max_price
FROM nonequi_items i
RIGHT JOIN nonequi_offers o ON i.category = o.target_category AND i.price <= o.max_price
WHERE o.description @@@ 'exclusive OR budget'
ORDER BY o.id, i.id NULLS LAST
LIMIT 10;

-- =============================================================================
-- SECTION 4: Cross-Table Disjunctive Search (OR) with Non-Equi JOIN
-- =============================================================================

-- Test 4.1: OR across tables with non-equi condition
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name
FROM nonequi_items i
JOIN nonequi_offers o ON i.price <= o.max_price
WHERE i.description @@@ 'gaming' OR o.description @@@ 'clearance'
ORDER BY i.id, o.id
LIMIT 10;

SELECT i.id, i.name, o.promo_name
FROM nonequi_items i
JOIN nonequi_offers o ON i.price <= o.max_price
WHERE i.description @@@ 'gaming' OR o.description @@@ 'clearance'
ORDER BY i.id, o.id
LIMIT 10;

-- =============================================================================
-- SECTION 5: Parity Checks (Custom Scan ON vs OFF)
-- =============================================================================

-- Test 5.1: Non-equi inner join parity
SET paradedb.enable_join_custom_scan = off;
SELECT i.id, i.name, o.promo_name
FROM nonequi_items i
JOIN nonequi_offers o ON i.price <= o.max_price
WHERE i.description @@@ 'ergonomic'
ORDER BY i.id, o.id
LIMIT 10;

SET paradedb.enable_join_custom_scan = on;
SELECT i.id, i.name, o.promo_name
FROM nonequi_items i
JOIN nonequi_offers o ON i.price <= o.max_price
WHERE i.description @@@ 'ergonomic'
ORDER BY i.id, o.id
LIMIT 10;

-- Test 5.2: Non-equi left join parity
SET paradedb.enable_join_custom_scan = off;
SELECT i.id, i.name, o.promo_name
FROM nonequi_items i
LEFT JOIN nonequi_offers o ON i.category = o.target_category AND i.price < o.min_price
WHERE i.description @@@ 'cable OR mat OR mouse'
ORDER BY i.id, o.id NULLS LAST
LIMIT 10;

SET paradedb.enable_join_custom_scan = on;
SELECT i.id, i.name, o.promo_name
FROM nonequi_items i
LEFT JOIN nonequi_offers o ON i.category = o.target_category AND i.price < o.min_price
WHERE i.description @@@ 'cable OR mat OR mouse'
ORDER BY i.id, o.id NULLS LAST
LIMIT 10;

-- =============================================================================
-- SECTION 6: Fallback / Rejection
-- =============================================================================

-- Add an unindexed column to nonequi_items
ALTER TABLE nonequi_items ADD COLUMN unindexed_weight FLOAT;
UPDATE nonequi_items SET unindexed_weight = 1.5 WHERE id = 101;
UPDATE nonequi_items SET unindexed_weight = 0.2 WHERE id = 102;

-- Test 6.1: Non-equi condition on unindexed column should fall back to native join
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, i.name, o.promo_name
FROM nonequi_items i
JOIN nonequi_offers o ON i.unindexed_weight < o.min_price
WHERE i.description @@@ 'ergonomic'
ORDER BY i.id, o.id
LIMIT 10;

SELECT i.id, i.name, o.promo_name
FROM nonequi_items i
JOIN nonequi_offers o ON i.unindexed_weight < o.min_price
WHERE i.description @@@ 'ergonomic'
ORDER BY i.id, o.id
LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================

DROP TABLE IF EXISTS nonequi_items CASCADE;
DROP TABLE IF EXISTS nonequi_offers CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;

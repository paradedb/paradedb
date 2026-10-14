-- A prepared statement whose plan PostgreSQL caches and runs again. The
-- Aggregate Scan must leave the cached plan in a state that the next run can
-- use.

\i common/common_setup.sql

CREATE TABLE prepared_agg_sales (
    id SERIAL PRIMARY KEY,
    region TEXT NOT NULL,
    rating INTEGER NOT NULL,
    amount FLOAT8 NOT NULL,
    tags TEXT[] NOT NULL
);

INSERT INTO prepared_agg_sales (region, rating, amount, tags)
SELECT
    (ARRAY['east', 'north', 'west'])[(g % 3) + 1],
    (g % 4) + 1,
    g,
    CASE WHEN g % 2 = 0 THEN ARRAY['new', 'sale'] ELSE ARRAY['new'] END
FROM generate_series(1, 60) g;

CREATE INDEX prepared_agg_sales_idx ON prepared_agg_sales
USING paradedb (id, (region::pdb.literal), rating, amount, (tags::pdb.literal));

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- SECTION 1: No parameter -> PostgreSQL caches the plan on the first run
-- =====================================================================

\echo 'Test 1.1: GROUP BY'
PREPARE prepared_agg_group AS
SELECT region, COUNT(*)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
GROUP BY region
ORDER BY region;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE prepared_agg_group;
EXECUTE prepared_agg_group;
EXECUTE prepared_agg_group;
EXECUTE prepared_agg_group;

\echo 'Test 1.2: aggregate in an expression'
PREPARE prepared_agg_wrapped AS
SELECT region, COALESCE(SUM(amount), 0) + 1 AS total, COUNT(*)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
GROUP BY region
ORDER BY region;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE prepared_agg_wrapped;
EXECUTE prepared_agg_wrapped;
EXECUTE prepared_agg_wrapped;
EXECUTE prepared_agg_wrapped;

\echo 'Test 1.3: ORDER BY an aggregate'
PREPARE prepared_agg_order AS
SELECT rating, COUNT(*), MAX(amount)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
GROUP BY rating
ORDER BY MAX(amount) DESC, rating;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE prepared_agg_order;
EXECUTE prepared_agg_order;
EXECUTE prepared_agg_order;
EXECUTE prepared_agg_order;

\echo 'Test 1.4: no GROUP BY, ORDER BY the aggregate'
PREPARE prepared_agg_scalar AS
SELECT COUNT(*)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
ORDER BY COUNT(*);

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE prepared_agg_scalar;
EXECUTE prepared_agg_scalar;
EXECUTE prepared_agg_scalar;
EXECUTE prepared_agg_scalar;

\echo 'Test 1.5: pdb.agg()'
PREPARE prepared_agg_custom AS
SELECT region, pdb.agg('{"avg": {"field": "amount"}}'::jsonb)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
GROUP BY region
ORDER BY region;

EXECUTE prepared_agg_custom;
EXECUTE prepared_agg_custom;
EXECUTE prepared_agg_custom;

\echo 'Test 1.6: UNNEST in the GROUP BY'
PREPARE prepared_agg_unnest AS
SELECT UNNEST(tags) AS tag, COUNT(*)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
GROUP BY tag
ORDER BY tag;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE prepared_agg_unnest;
EXECUTE prepared_agg_unnest;
EXECUTE prepared_agg_unnest;
EXECUTE prepared_agg_unnest;

-- =====================================================================
-- SECTION 2: Parameters
-- =====================================================================

\echo 'Test 2.1: generic plan'
SET plan_cache_mode = force_generic_plan;

PREPARE prepared_agg_param(int) AS
SELECT region, COUNT(*), SUM(amount)
FROM prepared_agg_sales
WHERE rating = $1 AND id @@@ pdb.all()
GROUP BY region
ORDER BY region;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE prepared_agg_param(1);
EXECUTE prepared_agg_param(1);
EXECUTE prepared_agg_param(2);
EXECUTE prepared_agg_param(99);
EXECUTE prepared_agg_param(1);

RESET plan_cache_mode;

\echo 'Test 2.2: default plan cache mode, more runs than PostgreSQL makes custom plans for'
PREPARE prepared_agg_default(int) AS
SELECT region, COUNT(*), SUM(amount)
FROM prepared_agg_sales
WHERE rating = $1 AND id @@@ pdb.all()
GROUP BY region
ORDER BY region;

EXECUTE prepared_agg_default(1);
EXECUTE prepared_agg_default(2);
EXECUTE prepared_agg_default(3);
EXECUTE prepared_agg_default(4);
EXECUTE prepared_agg_default(1);
EXECUTE prepared_agg_default(2);
EXECUTE prepared_agg_default(3);
EXECUTE prepared_agg_default(4);

-- =====================================================================
-- SECTION 3: A statement in a function
-- =====================================================================
-- PL/pgSQL caches the plan of each statement. The function calls itself in
-- its loop, so the plan runs again while an earlier run is still open.

CREATE FUNCTION prepared_agg_walk(depth int) RETURNS SETOF text
LANGUAGE plpgsql AS $$
DECLARE
    row record;
BEGIN
    FOR row IN
        SELECT region, COUNT(*) AS count
        FROM prepared_agg_sales
        WHERE id @@@ pdb.all()
        GROUP BY region
        ORDER BY region
    LOOP
        RETURN NEXT depth || ':' || row.region || ':' || row.count;
        IF depth < 1 AND row.region = 'east' THEN
            RETURN QUERY SELECT prepared_agg_walk(depth + 1);
        END IF;
    END LOOP;
END;
$$;

SELECT prepared_agg_walk(0);
SELECT prepared_agg_walk(0);

DROP FUNCTION prepared_agg_walk(int);

-- =====================================================================
-- SECTION 4: Same results from PostgreSQL
-- =====================================================================

SET paradedb.enable_aggregate_custom_scan TO off;

SELECT region, COUNT(*)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
GROUP BY region
ORDER BY region;

SELECT region, COALESCE(SUM(amount), 0) + 1 AS total, COUNT(*)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
GROUP BY region
ORDER BY region;

SELECT rating, COUNT(*), MAX(amount)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
GROUP BY rating
ORDER BY MAX(amount) DESC, rating;

SELECT region, COUNT(*), SUM(amount)
FROM prepared_agg_sales
WHERE rating = 1 AND id @@@ pdb.all()
GROUP BY region
ORDER BY region;

SELECT region, COUNT(*), SUM(amount)
FROM prepared_agg_sales
WHERE rating = 2 AND id @@@ pdb.all()
GROUP BY region
ORDER BY region;

SELECT UNNEST(tags) AS tag, COUNT(*)
FROM prepared_agg_sales
WHERE id @@@ pdb.all()
GROUP BY tag
ORDER BY tag;

SELECT COUNT(*)
FROM prepared_agg_sales
WHERE id @@@ pdb.all();

DEALLOCATE ALL;
RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE prepared_agg_sales;

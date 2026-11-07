-- Regression test for https://github.com/paradedb/paradedb/issues/4596
-- pdb.score() and pdb.snippet() with a SELECT subquery in WHERE clause
-- should use ParadeDB Base Scan, not Index Scan

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS animals CASCADE;

CREATE TABLE animals (
    id SERIAL PRIMARY KEY,
    description TEXT
);

INSERT INTO animals (description) VALUES
('description 1 dog'),
('description 2 cat'),
('description 3 dog'),
('description 4 parrot');

CREATE INDEX animals_idx ON animals
USING paradedb (id, description);

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan = OFF;

-- Test 1: pdb.score() with (SELECT true) should use ParadeDB Base Scan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id) AS score
FROM animals
WHERE description ||| 'dog' AND (SELECT true)
LIMIT 1;

SELECT id, pdb.score(id) AS score
FROM animals
WHERE description ||| 'dog' AND (SELECT true)
ORDER BY score DESC
LIMIT 1;

-- Test 2: pdb.snippet() with (SELECT true) should use ParadeDB Base Scan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.snippet(description) AS snippet
FROM animals
WHERE description ||| 'dog' AND (SELECT true)
LIMIT 1;

SELECT id, pdb.snippet(description) AS snippet
FROM animals
WHERE description ||| 'dog' AND (SELECT true)
ORDER BY id
LIMIT 1;

-- Test 3: (SELECT false) should return no rows
SELECT id, pdb.score(id) AS score
FROM animals
WHERE description ||| 'dog' AND (SELECT false)
ORDER BY score DESC
LIMIT 1;

-- Test 4: pdb.score() with a function subquery (RLS policy pattern)
CREATE FUNCTION has_access() RETURNS boolean LANGUAGE sql STABLE AS $$ SELECT true $$;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id) AS score
FROM animals
WHERE description ||| 'dog' AND (SELECT has_access())
LIMIT 1;

SELECT id, pdb.score(id) AS score
FROM animals
WHERE description ||| 'dog' AND (SELECT has_access())
ORDER BY score DESC
LIMIT 1;

DROP FUNCTION has_access();

-- Test 5: pdb.score() with a table lookup subquery
CREATE TABLE config (name TEXT, value BOOLEAN);
INSERT INTO config VALUES ('enabled', true);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id) AS score
FROM animals
WHERE description ||| 'dog' AND (SELECT value FROM config WHERE name = 'enabled' LIMIT 1)
LIMIT 1;

SELECT id, pdb.score(id) AS score
FROM animals
WHERE description ||| 'dog' AND (SELECT value FROM config WHERE name = 'enabled' LIMIT 1)
ORDER BY score DESC
LIMIT 1;

DROP TABLE config;

-- =============================================================================
-- Test 6: JoinScan with pdb.score() combined with an InitPlan (`PARAM_EXEC`) or
-- prepared parameter (`PARAM_EXTERN`) in the SELECT target list.
--
-- `pdb.score()` combined with an `InitPlan` param (`6a`) or `$1` (`6b`) is
-- projected by Postgres above the scan, not wrapped in a `PgExprUdf`.
-- =============================================================================
CREATE TABLE shelters (
    id INT PRIMARY KEY,
    animal_id INT,
    name TEXT
);

INSERT INTO shelters (id, animal_id, name) VALUES
(10, 1, 'downtown dog rescue'),
(20, 2, 'uptown cat haven'),
(30, 3, 'westside dog shelter'),
(40, 4, 'avian sanctuary');

CREATE INDEX shelters_idx ON shelters
USING paradedb (id, animal_id, name);

SET paradedb.enable_join_custom_scan = on;
SET paradedb.rank_join_min_rows = 0;

-- 6a: InitPlan (`PARAM_EXEC`) combined with `pdb.score()` in the SELECT list of a JoinScan
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT a.id,
       pdb.score(a.id) - (SELECT 1.0::real WHERE current_setting('server_version_num') <> '') AS score
FROM animals a
JOIN shelters s ON s.animal_id = a.id
WHERE a.description ||| 'dog' OR s.name ||| 'cat'
ORDER BY pdb.score(a.id) + pdb.score(s.id) DESC, a.id
LIMIT 5;

SELECT a.id,
       pdb.score(a.id) - (SELECT 1.0::real WHERE current_setting('server_version_num') <> '') AS score
FROM animals a
JOIN shelters s ON s.animal_id = a.id
WHERE a.description ||| 'dog' OR s.name ||| 'cat'
ORDER BY pdb.score(a.id) + pdb.score(s.id) DESC, a.id
LIMIT 5;

-- 6b: Generic prepared-statement parameter (`PARAM_EXTERN`) combined with `pdb.score()`
-- in the SELECT list of a JoinScan
SET plan_cache_mode = force_generic_plan;

PREPARE join_score_param(real) AS
SELECT a.id,
       pdb.score(a.id) - $1 AS score
FROM animals a
JOIN shelters s ON s.animal_id = a.id
WHERE a.description ||| 'dog' OR s.name ||| 'cat'
ORDER BY pdb.score(a.id) + pdb.score(s.id) DESC, a.id
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
EXECUTE join_score_param(1.0);

EXECUTE join_score_param(1.0);

DEALLOCATE join_score_param;
RESET plan_cache_mode;
RESET paradedb.rank_join_min_rows;
RESET paradedb.enable_join_custom_scan;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;

DROP TABLE IF EXISTS shelters CASCADE;
DROP TABLE IF EXISTS animals CASCADE;

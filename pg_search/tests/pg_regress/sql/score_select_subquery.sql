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
USING bm25 (id, description)
WITH (key_field = 'id');

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

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;

DROP TABLE IF EXISTS animals CASCADE;
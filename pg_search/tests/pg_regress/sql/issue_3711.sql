CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS ints;
CREATE TABLE ints (id SERIAL PRIMARY KEY, i integer, j integer);
INSERT INTO ints (i, j) VALUES (1, 2), (2, 3), (3, 4);
CREATE INDEX idx_ints ON ints USING bm25 (id, ((i * 2)::pdb.alias('another_name'))) with (key_field = 'id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM ints WHERE (i * 2) = 2 AND id @@@ pdb.all();

SELECT * FROM ints WHERE (i * 2) = 2 AND id @@@ pdb.all();
DROP INDEX idx_ints;

CREATE OR REPLACE FUNCTION add_two_numbers(a integer, b integer)
RETURNS integer
LANGUAGE sql
IMMUTABLE
RETURNS NULL ON NULL INPUT
AS $$
    SELECT a + b;
$$;

CREATE INDEX idx_ints ON ints USING bm25 (id, (add_two_numbers(i, j)::pdb.alias('another_name'))) with (key_field = 'id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM ints WHERE add_two_numbers(i, j) = 5 AND id @@@ pdb.all();

SELECT * FROM ints WHERE add_two_numbers(i, j) = 5 AND id @@@ pdb.all();

DROP INDEX idx_ints;
DROP FUNCTION add_two_numbers;
DROP TABLE ints;

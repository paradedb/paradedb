-- paradedb.max_term_set_size: a term set over the limit errors, and one over
-- half the limit warns.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS term_set_size_cap;
CREATE TABLE term_set_size_cap (id int, description text, rating int);
INSERT INTO term_set_size_cap
SELECT i, 'item ' || i, i % 10 FROM generate_series(1, 100) i;
CREATE INDEX term_set_size_cap_idx ON term_set_size_cap
USING bm25 (id, description, rating);

SHOW paradedb.max_term_set_size;

SET paradedb.max_term_set_size = 4;

-- At or below half the limit: no warning.
SELECT count(*) FROM term_set_size_cap
WHERE description @@@ pdb.term_set(ARRAY['1', '2']);

-- Above half the limit: warning.
SELECT count(*) FROM term_set_size_cap
WHERE description @@@ pdb.term_set(ARRAY['1', '2', '3']);

-- At the limit: warning.
SELECT count(*) FROM term_set_size_cap
WHERE description === ARRAY['1', '2', '3', '4'];

-- Warns once per statement, even when the query is built in parallel workers.
SET debug_parallel_query = on;
SELECT count(*) FROM term_set_size_cap
WHERE description @@@ pdb.term_set(ARRAY['1', '2', '3']);
RESET debug_parallel_query;

-- planner_warnings = 'off' silences the warning.
SET paradedb.planner_warnings = 'off';
SELECT count(*) FROM term_set_size_cap
WHERE description @@@ pdb.term_set(ARRAY['1', '2', '3']);
RESET paradedb.planner_warnings;

-- Above the limit: error.
SELECT count(*) FROM term_set_size_cap
WHERE description @@@ pdb.term_set(ARRAY['1', '2', '3', '4', '5']);

SELECT count(*) FROM term_set_size_cap
WHERE id @@@ pdb.all() AND rating IN (1, 2, 3, 4, 5);

SELECT count(*) FROM term_set_size_cap
WHERE id @@@ paradedb.term_set(terms => ARRAY[
    paradedb.term(field => 'rating', value => 1),
    paradedb.term(field => 'rating', value => 2),
    paradedb.term(field => 'rating', value => 3),
    paradedb.term(field => 'rating', value => 4),
    paradedb.term(field => 'rating', value => 5)
]);

RESET paradedb.max_term_set_size;
DROP TABLE term_set_size_cap;

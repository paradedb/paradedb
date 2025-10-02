DROP TABLE IF EXISTS missing_fn;
CREATE TABLE missing_fn (id int);
INSERT INTO missing_fn (id) SELECT generate_series(1, 1000);
CREATE INDEX idxmissing_fn ON missing_fn USING bm25 (id) WITH (key_field = 'id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM missing_fn WHERE id = ANY(ARRAY[3]) AND id @@@ paradedb.all() ORDER BY id;
SELECT * FROM missing_fn WHERE id = ANY(ARRAY[3]) AND id @@@ paradedb.all() ORDER BY id;

BEGIN;
ALTER EXTENSION pg_search DROP FUNCTION paradedb.terms_with_operator;
DROP FUNCTION paradedb.terms_with_operator;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM missing_fn WHERE id = ANY(ARRAY[3]) AND id @@@ paradedb.all() ORDER BY id;
SELECT * FROM missing_fn WHERE id = ANY(ARRAY[3]) AND id @@@ paradedb.all() ORDER BY id;
ABORT;

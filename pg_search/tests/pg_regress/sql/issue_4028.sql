\i common/common_setup.sql

CREATE TABLE test (id serial PRIMARY KEY, description text);

INSERT INTO test (description) VALUES ('Cloud Engagement Manager'),('cloud engineer'), ('Clōüd engineer'),('cloud Engineer'), ('Cloud engineer');

CREATE INDEX test_bm25 ON test
USING bm25 (
	id,
	(lower(description)::pdb.normalized('ascii_folding=true'))
) WITH (key_field = id);

SELECT * FROM paradedb.schema('test_bm25');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM test
WHERE lower(description) === 'cloud engineer'
ORDER BY lower(description)
LIMIT 10;

SELECT * FROM test
WHERE lower(description) === 'cloud engineer'
ORDER BY lower(description)
LIMIT 10;

DROP TABLE test;

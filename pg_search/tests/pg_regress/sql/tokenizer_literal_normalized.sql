\i common/common_setup.sql

CREATE TABLE test_table (
    id SERIAL PRIMARY KEY,
    text TEXT
);

INSERT INTO test_table (text) VALUES
    ('Hello, world!'),
    ('Hello, world!');

CREATE INDEX idx_test_table ON test_table USING bm25 (id, (text::pdb.unicode_words('ascii_folding=true'))) WITH (key_field='id');
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text, pdb.agg('{"value_count": {"field": "id"}}') FROM test_table WHERE id @@@ pdb.all() GROUP BY text ORDER BY text LIMIT 5;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM test_table WHERE id @@@ pdb.all() ORDER BY text LIMIT 5;
DROP INDEX idx_test_table;

CREATE INDEX idx_test_table ON test_table USING bm25 (id, (text::pdb.normalized('ascii_folding=true'))) WITH (key_field='id');
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text, pdb.agg('{"value_count": {"field": "id"}}') FROM test_table WHERE id @@@ pdb.all() GROUP BY text ORDER BY text LIMIT 5;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM test_table WHERE id @@@ pdb.all() ORDER BY text LIMIT 5;
DROP INDEX idx_test_table;

CREATE INDEX idx_test_table ON test_table USING bm25 (id, (text::pdb.literal)) WITH (key_field='id');
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT text, pdb.agg('{"value_count": {"field": "id"}}') FROM test_table WHERE id @@@ pdb.all() GROUP BY text ORDER BY text LIMIT 5;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM test_table WHERE id @@@ pdb.all() ORDER BY text LIMIT 5;

DROP TABLE test_table;

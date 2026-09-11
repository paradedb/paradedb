CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS index_json;
CREATE TABLE index_json(
    id serial8 not null primary key,
    j json,
    jb jsonb
);
INSERT INTO index_json (j, jb) VALUES ('{"key1": "value1"}', '{"key2": "value2"}');
CREATE INDEX idxindex_json ON index_json USING paradedb (id, j, jb);

SELECT * FROM paradedb.schema('idxindex_json') ORDER BY name;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' @@@ 'value1';
SELECT * FROM index_json WHERE j->'key1' @@@ 'value1';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' &&& 'value1';
SELECT * FROM index_json WHERE j->'key1' &&& 'value1';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' ||| 'value1';
SELECT * FROM index_json WHERE j->'key1' ||| 'value1';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' ### 'value1';
SELECT * FROM index_json WHERE j->'key1' ### 'value1';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' === 'value1';
SELECT * FROM index_json WHERE j->'key1' === 'value1';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' @@@ 'value2';
SELECT * FROM index_json WHERE jb->'key2' @@@ 'value2';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' &&& 'value2';
SELECT * FROM index_json WHERE jb->'key2' &&& 'value2';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' ||| 'value2';
SELECT * FROM index_json WHERE jb->'key2' ||| 'value2';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' ### 'value2';
SELECT * FROM index_json WHERE jb->'key2' ### 'value2';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' === 'value2';
SELECT * FROM index_json WHERE jb->'key2' === 'value2';

DROP INDEX idxindex_json;
CREATE INDEX idxindex_json ON index_json USING paradedb (id, (j::pdb.ngram(2, 3)), (jb::pdb.whitespace));

SELECT * FROM paradedb.schema('idxindex_json') ORDER BY name;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' @@@ 'value1';
SELECT * FROM index_json WHERE j->'key1' @@@ 'value1';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' &&& 'value1';
SELECT * FROM index_json WHERE j->'key1' &&& 'value1';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' ||| 'value1';
SELECT * FROM index_json WHERE j->'key1' ||| 'value1';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' ### 'value1';
SELECT * FROM index_json WHERE j->'key1' ### 'value1';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE j->'key1' === 'value1';
SELECT * FROM index_json WHERE j->'key1' === 'value1';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' @@@ 'value2';
SELECT * FROM index_json WHERE jb->'key2' @@@ 'value2';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' &&& 'value2';
SELECT * FROM index_json WHERE jb->'key2' &&& 'value2';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' ||| 'value2';
SELECT * FROM index_json WHERE jb->'key2' ||| 'value2';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' ### 'value2';
SELECT * FROM index_json WHERE jb->'key2' ### 'value2';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM index_json WHERE jb->'key2' === 'value2';
SELECT * FROM index_json WHERE jb->'key2' === 'value2';

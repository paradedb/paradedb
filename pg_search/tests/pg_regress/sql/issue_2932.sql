\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX on mock_items USING bm25 (id, description, rating) WITH (key_field='id');

SELECT description, paradedb.score(id) * 2 AS score FROM mock_items WHERE description @@@ 'shoes' ORDER BY score DESC LIMIT 3;
SELECT description, rating, paradedb.score(id) * rating AS score FROM mock_items WHERE description @@@ 'shoes' OR rating > 2 ORDER BY score DESC, rating LIMIT 3;
SELECT description, rating, paradedb.score(id) AS score, paradedb.score(id) * rating AS score_times_rating FROM mock_items WHERE description @@@ 'shoes' OR rating > 2 ORDER BY score_times_rating DESC LIMIT 3;
SELECT description, rating, paradedb.score(id) AS score, paradedb.score(id) * rating AS score_times_rating FROM mock_items WHERE description @@@ 'shoes' OR rating > 2 ORDER BY score DESC LIMIT 3;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, paradedb.score(id) AS score FROM mock_items WHERE description @@@ 'shoes' ORDER BY score DESC LIMIT 3;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, paradedb.score(id) * 2 AS score FROM mock_items WHERE description @@@ 'shoes' ORDER BY score DESC LIMIT 3;

DROP TABLE mock_items;

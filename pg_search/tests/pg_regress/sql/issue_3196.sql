\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX on mock_items USING bm25 (id, description, rating, category, metadata) WITH (key_field='id', json_fields = '{"metadata": {"fast": true, "tokenizer": {"type": "keyword"}}}');

INSERT INTO mock_items (rating, metadata) VALUES (null, null), (null, null);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM mock_items WHERE id @@@ paradedb.all();

SELECT COUNT(*) FROM mock_items WHERE id @@@ paradedb.all();

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(rating) FROM mock_items WHERE id @@@ paradedb.all();

SELECT COUNT(rating) FROM mock_items WHERE id @@@ paradedb.all();

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(metadata->>'color') FROM mock_items WHERE id @@@ paradedb.all();

SELECT COUNT(metadata->>'color') FROM mock_items WHERE id @@@ paradedb.all();

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(COALESCE(rating, 0)) FROM mock_items WHERE id @@@ paradedb.all();

SELECT COUNT(COALESCE(rating, 0)) FROM mock_items WHERE id @@@ paradedb.all();

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(COALESCE(metadata->>'color', 'red')) FROM mock_items WHERE id @@@ paradedb.all();

SELECT COUNT(COALESCE(metadata->>'color', 'red')) FROM mock_items WHERE id @@@ paradedb.all();

DROP TABLE mock_items;

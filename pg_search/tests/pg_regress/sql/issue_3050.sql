\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX on mock_items USING bm25 (id, description, rating, category, metadata) WITH (key_field='id', json_fields = '{"metadata": {"fast": true, "tokenizer": {"type": "raw", "lowercase": true}}}');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, rating, COUNT(*)
FROM mock_items
WHERE id @@@ paradedb.all()
GROUP BY id, rating
ORDER BY id, rating
LIMIT 5;

SELECT id, rating, COUNT(*)
FROM mock_items
WHERE id @@@ paradedb.all()
GROUP BY id, rating
ORDER BY id, rating
LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, metadata->>'color' AS color, COUNT(*)
FROM mock_items
WHERE id @@@ paradedb.all()
GROUP BY id, metadata->>'color'
ORDER BY id, metadata->>'color'
LIMIT 5;

SELECT id, metadata->>'color' AS color, COUNT(*)
FROM mock_items
WHERE id @@@ paradedb.all()
GROUP BY id, metadata->>'color'
ORDER BY id, metadata->>'color'
LIMIT 5;

DROP TABLE mock_items;

\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX on mock_items USING bm25 (id, description, rating, category, metadata) WITH (key_field='id', json_fields = '{"metadata": {"fast": true, "tokenizer": {"type": "raw", "lowercase": true}}}');
SELECT id, description @@@ 'shoes' FROM mock_items ORDER BY id;

DROP TABLE mock_items;

\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
WITH (key_field='id', text_fields='{"description": {"tokenizer": {"type": "keyword"}}}');

SELECT * FROM paradedb.schema('search_idx');
SELECT * FROM mock_items WHERE id @@@ paradedb.exists('description') ORDER BY id LIMIT 5;

DROP TABLE mock_items;

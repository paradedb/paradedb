\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, (metadata->>'color'))
WITH (key_field='id');

EXPLAIN SELECT COUNT(*) FROM mock_items WHERE metadata->>'color' @@@ 'white';
SELECT COUNT(*) FROM mock_items WHERE metadata->>'color' @@@ 'white';

DROP TABLE mock_items;

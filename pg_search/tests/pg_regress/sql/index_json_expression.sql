\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, ((metadata->>'color')::pdb.ngram(2, 3)))
WITH (key_field='id');

SELECT * FROM paradedb.schema('search_idx') ORDER BY name;

EXPLAIN SELECT COUNT(*) FROM mock_items WHERE metadata->>'color' @@@ 'white';
SELECT COUNT(*) FROM mock_items WHERE metadata->>'color' @@@ 'white';

DROP TABLE mock_items;
RESET paradedb.enable_aggregate_custom_scan;

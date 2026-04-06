\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, (metadata::pdb.literal))
WITH (key_field='id');

SELECT pdb.agg('{"terms": {"field": "metadata.color"}}')
FROM mock_items
WHERE id @@@ pdb.all();

DROP TABLE mock_items;

\i common/common_setup.sql

CALL paradedb.create_paradedb_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX on mock_items USING paradedb (id, description, rating, category, (metadata::pdb.literal_normalized('lowercase=true')));
SELECT id, description ||| 'shoes' FROM mock_items ORDER BY id;

DROP TABLE mock_items;

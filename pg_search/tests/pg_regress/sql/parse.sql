\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX on mock_items
USING bm25 (id, description, rating, category, metadata)
WITH (key_field='id');

SELECT id, description, category FROM mock_items
WHERE id @@@ pdb.parse('description:(running shoes) AND category:footwear');

SELECT id, description, category FROM mock_items
WHERE id @@@ pdb.parse('description:(running shoes) AND category:footwear', conjunction_mode => true);

SELECT id, description, category FROM mock_items
WHERE description @@@ pdb.parse_with_field('(running shoes)', lenient => true);

DROP TABLE mock_items;

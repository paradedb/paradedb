\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX on mock_items
USING bm25 (id, description, rating, category, metadata, created_at, last_updated_date, latest_available_time)
WITH (key_field='id');

SELECT id, description, category FROM mock_items
WHERE id @@@ pdb.parse('description:(running shoes) AND category:footwear');

SELECT id, description, category FROM mock_items
WHERE id @@@ pdb.parse('description:(running shoes) AND category:footwear', conjunction_mode => true);

SELECT id, description, category FROM mock_items
WHERE description @@@ pdb.parse_with_field('(running shoes)', lenient => true);

SELECT id, description, created_at FROM mock_items
WHERE id @@@ pdb.parse('created_at:"2023-05-01 09:12:34"') ORDER BY id;

SELECT id, description, last_updated_date FROM mock_items
WHERE id @@@ pdb.parse('last_updated_date:"2023-05-03"') ORDER BY id;

SELECT id, description, latest_available_time FROM mock_items
WHERE id @@@ pdb.parse('latest_available_time:"09:12:34"') ORDER BY id;

DROP TABLE mock_items;

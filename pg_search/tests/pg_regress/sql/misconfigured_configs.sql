CREATE EXTENSION IF NOT EXISTS pg_search;

CALL paradedb.create_paradedb_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING paradedb (id, (description::pdb.literal), category, rating, in_stock, created_at, metadata, weight_range);

SELECT description, rating, category
FROM mock_items
WHERE description === 'Sleek running shoes'
LIMIT 5;

DROP TABLE mock_items;

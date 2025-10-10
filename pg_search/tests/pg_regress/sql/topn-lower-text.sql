\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, (lower(description)::pdb.literal), rating)
WITH (key_field='id');

-- This gets a TopN scan
EXPLAIN SELECT description, rating FROM mock_items
WHERE description === 'sleek running shoes'
ORDER BY lower(description) DESC
LIMIT 5;

SELECT description, rating FROM mock_items
WHERE description === 'sleek running shoes'
ORDER BY lower(description) DESC
LIMIT 5;

-- No TopN scan
EXPLAIN SELECT description, rating FROM mock_items
WHERE description === 'sleek running shoes'
ORDER BY description DESC
LIMIT 5;

SELECT description, rating FROM mock_items
WHERE description === 'sleek running shoes'
ORDER BY description DESC
LIMIT 5;

DROP TABLE mock_items;

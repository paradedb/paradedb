\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, description, rating)
WITH (key_field='id', text_fields='{"description": {"fast": true, "normalizer": "lowercase"}}');

-- This gets a TopN scan
EXPLAIN SELECT description, rating FROM mock_items
WHERE description @@@ 'shoes'
ORDER BY lower(description) DESC
LIMIT 5;

SELECT description, rating FROM mock_items
WHERE description @@@ 'shoes'
ORDER BY lower(description) DESC
LIMIT 5;

-- No TopN scan
EXPLAIN SELECT description, rating FROM mock_items
WHERE description @@@ 'shoes'
ORDER BY description DESC
LIMIT 5;

SELECT description, rating FROM mock_items
WHERE description @@@ 'shoes'
ORDER BY description DESC
LIMIT 5;

DROP TABLE mock_items;

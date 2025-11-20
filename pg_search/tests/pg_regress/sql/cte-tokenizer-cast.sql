\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

-- Test 1: Create index with tokenizer cast
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple), category, rating, in_stock, created_at, metadata, weight_range)
WITH (key_field='id');

-- Test 2: Direct query (should work)
SELECT id, description FROM mock_items WHERE description ||| 'shoes' ORDER BY id;

-- Test 3: CTE without LIMIT or ORDER BY (currently fails)
WITH q AS (
  SELECT * FROM mock_items WHERE description ||| 'shoes'
)
SELECT id, description FROM q ORDER BY id;

-- Test 4: CTE with LIMIT (should work)
WITH q AS (
  SELECT * FROM mock_items WHERE description ||| 'shoes' LIMIT 10
)
SELECT id, description FROM q ORDER BY id;

-- Test 5: CTE with ORDER BY (should work)
WITH q AS (
  SELECT * FROM mock_items WHERE description ||| 'shoes' ORDER BY rating
)
SELECT id, description FROM q ORDER BY id;

-- Clean up
DROP TABLE mock_items CASCADE;


\i common/common_setup.sql

-- Test CTE queries with tokenizer cast in index definition
CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

-- Create index with tokenizer cast on description field
CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple), category, rating, in_stock, created_at, metadata, weight_range)
WITH (key_field='id');

-- Test 1: Direct query
SELECT id, description FROM mock_items WHERE description ||| 'shoes' ORDER BY id;

-- Test 2: CTE without LIMIT or ORDER BY
WITH q AS (
  SELECT * FROM mock_items WHERE description ||| 'shoes'
)
SELECT id, description FROM q ORDER BY id;

-- Test 3: CTE with LIMIT
WITH q AS (
  SELECT * FROM mock_items WHERE description ||| 'shoes' LIMIT 10
)
SELECT id, description FROM q ORDER BY id;

-- Test 4: CTE with ORDER BY
WITH q AS (
  SELECT * FROM mock_items WHERE description ||| 'shoes' ORDER BY rating
)
SELECT id, description FROM q ORDER BY id;

-- Test 5: Nested CTE
WITH q1 AS (
  SELECT * FROM mock_items WHERE description ||| 'shoes'
),
q2 AS (
  SELECT * FROM q1 WHERE rating > 3
)
SELECT id, description, rating FROM q2 ORDER BY id;

-- Test 6: CTE with other operators
WITH q AS (
  SELECT * FROM mock_items WHERE description @@@ 'shoes'
)
SELECT id, description FROM q ORDER BY id;

WITH q AS (
  SELECT * FROM mock_items WHERE description &&& 'shoes'
)
SELECT id, description FROM q ORDER BY id;

WITH q AS (
  SELECT * FROM mock_items WHERE description === 'shoes'
)
SELECT id, description FROM q ORDER BY id;

-- Clean up
DROP TABLE mock_items CASCADE;


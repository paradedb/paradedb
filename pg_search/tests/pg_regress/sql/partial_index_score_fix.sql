-- Test for partial index scoring with non-indexed predicates
-- This tests the fix for using partial index predicates instead of All query for HeapExpr

CREATE EXTENSION IF NOT EXISTS pg_search;

-- Setup test table
CREATE TABLE partial_test (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    rating INTEGER
);

INSERT INTO partial_test (description, category, rating) VALUES
('Apple iPhone', 'Electronics', 5),
('Samsung Galaxy', 'Electronics', 4),
('Nike Shoes', 'Footwear', 3),
('Apple Watch', 'Electronics', 4),
('Adidas Sneakers', 'Footwear', 2);

-- Create partial index with WHERE clause
CREATE INDEX partial_test_idx ON partial_test 
USING bm25 (id, description)
WITH (key_field = 'id')
WHERE category = 'Electronics';

-- Test Case 1: Query with only indexed field - should work correctly
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, category, paradedb.score(id) as score
FROM partial_test 
WHERE description @@@ 'Apple'
ORDER BY score DESC;

SELECT id, description, category, paradedb.score(id) as score
FROM partial_test 
WHERE description @@@ 'Apple'
ORDER BY score DESC;

-- Test Case 2: Query with indexed field + non-indexed predicate
-- This should use the partial index predicate (category = 'Electronics') 
-- instead of All query for the non-indexed rating filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, category, rating, paradedb.score(id) as score
FROM partial_test 
WHERE description @@@ 'Apple' AND rating >= 4
ORDER BY score DESC;

SELECT id, description, category, rating, paradedb.score(id) as score
FROM partial_test 
WHERE description @@@ 'Apple' AND rating >= 4
ORDER BY score DESC;

-- Test Case 3: Query with only non-indexed predicate
-- This should still use the partial index predicate for the base query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, category, rating, paradedb.score(id) as score
FROM partial_test 
WHERE rating >= 4
ORDER BY score DESC;

SELECT id, description, category, rating, paradedb.score(id) as score
FROM partial_test 
WHERE rating >= 4
ORDER BY score DESC;

-- Cleanup
DROP INDEX partial_test_idx;
DROP TABLE partial_test;

-- Test case for partial index scoring fix
-- This reproduces the issue seen in bm25_partial_index_search and bm25_partial_index_hybrid tests

-- Setup test table
CALL paradedb.create_bm25_test_table(table_name => 'test_partial_index', schema_name => 'paradedb');

-- Create partial index with predicate WHERE category = 'Electronics'
CREATE INDEX partial_idx ON paradedb.test_partial_index
USING bm25 (id, description, category, rating)
WITH (
    key_field = 'id',
    text_fields = '{
        "description": {
            "tokenizer": {"type": "en_stem"}
        }
    }'
) WHERE category = 'Electronics';

-- Test 1: Initial query should return only Electronics items with rating > 1
-- This should return 5 results (all Electronics with rating > 1)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

SELECT description, rating, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

-- Insert test data
INSERT INTO paradedb.test_partial_index (description, category, rating, in_stock) VALUES
('Product 1', 'Electronics', 2, true),
('Product 2', 'Electronics', 1, false),
('Product 3', 'Footwear', 2, true);

-- Test 2: After insert, should return 6 results (5 original + 1 new Electronics with rating > 1)
-- The key insight: Product 3 (Footwear) should NOT be returned since it's not in the partial index
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

SELECT description, rating, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

-- Test 3: Update Product 1 to Footwear - should reduce results to 5
UPDATE paradedb.test_partial_index SET category = 'Footwear' WHERE description = 'Product 1';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

SELECT description, rating, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

-- Test 4: Update Product 3 to Electronics - should increase results to 6  
UPDATE paradedb.test_partial_index SET category = 'Electronics' WHERE description = 'Product 3';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

SELECT description, rating, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

-- Test 5: Verify that non-Electronics items are not returned even if they match the query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'category:Footwear AND rating:>1'
ORDER BY rating LIMIT 20;

SELECT description, category 
FROM paradedb.test_partial_index
WHERE test_partial_index @@@ 'category:Footwear AND rating:>1'
ORDER BY rating LIMIT 20;

-- Cleanup
DROP INDEX partial_idx;
DROP TABLE paradedb.test_partial_index; 

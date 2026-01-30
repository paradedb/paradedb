-- Test for partial index behavior
-- A partial index can only be used when the query predicates IMPLY the partial index predicate.
-- This test verifies correct behavior with predicate_implied_by checking.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.global_mutable_segment_rows = 0;

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
('Apple Box', 'Footwear', 4),
('Adidas Sneakers', 'Footwear', 2);

-- Create partial index with WHERE clause
CREATE INDEX partial_test_idx ON partial_test
USING bm25 (id, description)
WITH (key_field = 'id')
WHERE category = 'Electronics';

-- ============================================================
-- Test Case 1: Query WITH partial index predicate
-- Query includes category = 'Electronics', so partial index CAN be used
-- ============================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, category, pdb.score(id) as score
FROM partial_test
WHERE description @@@ 'Apple' AND category = 'Electronics'
ORDER BY score DESC;

SELECT id, description, category, pdb.score(id) as score
FROM partial_test
WHERE description @@@ 'Apple' AND category = 'Electronics'
ORDER BY score DESC;

-- ============================================================
-- Test Case 2: Query WITH partial index predicate + additional filter
-- Query includes category = 'Electronics', so partial index CAN be used
-- The rating filter is evaluated as HeapExpr
-- ============================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, category, rating, pdb.score(id) as score
FROM partial_test
WHERE description @@@ 'Apple' AND category = 'Electronics' AND rating >= 4
ORDER BY score DESC;

SELECT id, description, category, rating, pdb.score(id) as score
FROM partial_test
WHERE description @@@ 'Apple' AND category = 'Electronics' AND rating >= 4
ORDER BY score DESC;

-- ============================================================
-- Test Case 3: Query with only partial index predicate (no @@@)
-- Query includes category = 'Electronics', so partial index CAN be used
-- ============================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, category, rating
FROM partial_test
WHERE category = 'Electronics' AND rating >= 4
AND id @@@ paradedb.all()
ORDER BY rating DESC;

SELECT id, description, category, rating
FROM partial_test
WHERE category = 'Electronics' AND rating >= 4
AND id @@@ paradedb.all()
ORDER BY rating DESC;

-- Cleanup
DROP INDEX partial_test_idx;
DROP TABLE partial_test;

-- ============================================================
-- Test case for partial index with category predicate
-- ============================================================

-- Setup test table
CALL paradedb.create_bm25_test_table(table_name => 'test_partial_index', schema_name => 'paradedb');

-- Create partial index with predicate WHERE category = 'Electronics'
CREATE INDEX partial_idx ON paradedb.test_partial_index
USING bm25 (id, description, category, rating)
WITH (
    key_field = 'id',
    text_fields = '{
        "description": {
            "tokenizer": {"type": "default"}
        }
    }'
) WHERE category = 'Electronics';

-- Test 1: Query WITH category = 'Electronics' predicate
-- This should use the partial index and return only Electronics items with rating > 1
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating, category
FROM paradedb.test_partial_index
WHERE category = 'Electronics' AND test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

SELECT description, rating, category
FROM paradedb.test_partial_index
WHERE category = 'Electronics' AND test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

-- Insert test data
INSERT INTO paradedb.test_partial_index (description, category, rating, in_stock) VALUES
('Product 1', 'Electronics', 2, true),
('Product 2', 'Electronics', 1, false),
('Product 3', 'Footwear', 2, true);

-- Test 2: After insert, should return 6 results (5 original + 1 new Electronics with rating > 1)
-- Product 3 (Footwear) is NOT in partial index and NOT in query results
SELECT description, rating, category
FROM paradedb.test_partial_index
WHERE category = 'Electronics' AND test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

-- Test 3: Update Product 1 to Footwear - should reduce results to 5
UPDATE paradedb.test_partial_index SET category = 'Footwear' WHERE description = 'Product 1';

SELECT description, rating, category
FROM paradedb.test_partial_index
WHERE category = 'Electronics' AND test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

-- Test 4: Update Product 3 to Electronics - should increase results to 6
UPDATE paradedb.test_partial_index SET category = 'Electronics' WHERE description = 'Product 3';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, rating, category
FROM paradedb.test_partial_index
WHERE category = 'Electronics' AND test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

SELECT description, rating, category
FROM paradedb.test_partial_index
WHERE category = 'Electronics' AND test_partial_index @@@ 'rating:>1'
ORDER BY rating LIMIT 20;

-- Test 5: Query for Footwear items should return nothing from partial index
-- (partial index only contains Electronics)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, category
FROM paradedb.test_partial_index
WHERE category = 'Electronics' AND test_partial_index @@@ 'category:Footwear AND rating:>1'
ORDER BY rating LIMIT 20;

SELECT description, category
FROM paradedb.test_partial_index
WHERE category = 'Electronics' AND test_partial_index @@@ 'category:Footwear AND rating:>1'
ORDER BY rating LIMIT 20;

-- Cleanup
DROP TABLE paradedb.test_partial_index CASCADE;

-- ============================================================
-- Test case for partial index with IS NULL predicate
-- This tests the original bug report where deleted_at IS NULL
-- in a partial index was still generating a heap filter
-- ============================================================

CREATE TABLE profiles (
    id BIGINT PRIMARY KEY,
    headline TEXT,
    deleted_at TIMESTAMPTZ
);

INSERT INTO profiles (id, headline, deleted_at) VALUES
(1, 'Software Engineer', NULL),
(2, 'Data Scientist', NULL),
(3, 'Product Manager', NULL),
(4, 'Deleted Profile', '2024-01-01 00:00:00'),
(5, 'DevOps Engineer', NULL);

-- Create partial index with WHERE deleted_at IS NULL
CREATE INDEX profiles_search_idx ON profiles
USING bm25 (id, headline)
WITH (key_field = 'id')
WHERE deleted_at IS NULL;

-- Test: Query with deleted_at IS NULL should NOT have heap_filter for this predicate
-- The partial index already guarantees deleted_at IS NULL, so no heap filter needed
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, headline, pdb.score(id) as score
FROM profiles
WHERE headline @@@ 'Engineer' AND deleted_at IS NULL
ORDER BY score DESC;

SELECT id, headline, pdb.score(id) as score
FROM profiles
WHERE headline @@@ 'Engineer' AND deleted_at IS NULL
ORDER BY score DESC;

-- Cleanup
DROP TABLE profiles CASCADE;

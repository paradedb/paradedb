-- Test that LIMIT clause uses TopN execution with mixed fast fields
-- This test ensures that when a LIMIT clause is used with mixed fast fields,
-- the execution uses the optimized TopN execution path

\i common/mixedff_advanced_setup.sql

\echo 'Test: LIMIT clause with TopN execution'

-- Create test table with mixed field types
DROP TABLE IF EXISTS limit_topn_test;
CREATE TABLE limit_topn_test (
    id SERIAL PRIMARY KEY,
    title TEXT,
    description TEXT,
    rating FLOAT,
    price FLOAT,
    category TEXT,
    is_available BOOLEAN,
    tags TEXT[],
    created_at TIMESTAMP
);

-- Insert test data with deterministic values
INSERT INTO limit_topn_test (title, description, rating, price, category, is_available, created_at)
SELECT
    'Product ' || i,
    'Description for product ' || i,
    (i % 5)::float + 1.0,  -- Deterministic ratings 1.0-5.0
    (100 * i)::float,    -- Deterministic prices 100, 200, 300, etc.
    (ARRAY['Electronics', 'Books', 'Clothing', 'Food', 'Toys'])[1 + (i % 5)],
    i % 2 = 0,             -- Deterministic boolean pattern
    '1988-04-29'::timestamp + ((i || ' days')::interval)
FROM generate_series(1, 100) i;

-- Create search index with multiple fast fields
DROP INDEX IF EXISTS limit_topn_idx;
CREATE INDEX limit_topn_idx ON limit_topn_test
USING bm25 (id, title, description, rating, price, category, is_available)
WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}, "fast": true}, "description": {"tokenizer": {"type": "default"}, "fast": true}, "category": {"tokenizer": {"type": "keyword"}, "fast": true}}',
    numeric_fields = '{"rating": {"fast": true}, "price": {"fast": true}}',
    boolean_fields = '{"is_available": {"fast": true}}'
);

-- Test basic LIMIT with mixed fields (should use TopN)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, rating, price, category
FROM limit_topn_test
WHERE title @@@ 'Product'
ORDER BY rating DESC
LIMIT 10;

-- Test LIMIT with mixed text and numeric fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, category, rating, price
FROM limit_topn_test
WHERE category @@@ 'Electronics'
ORDER BY price ASC
LIMIT 5;

SELECT title, category, rating, price
FROM limit_topn_test
WHERE category @@@ 'Electronics'
ORDER BY price ASC
LIMIT 5;

-- Test LIMIT with multiple string fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, category
FROM limit_topn_test
WHERE category @@@ 'Books OR Electronics'
ORDER BY title
LIMIT 15;

SELECT title, category
FROM limit_topn_test
WHERE category @@@ 'Books OR Electronics'
ORDER BY title
LIMIT 15;

-- Test LIMIT with boolean field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, is_available, rating
FROM limit_topn_test
WHERE is_available = true
ORDER BY rating DESC, title ASC
LIMIT 7;

SELECT title, is_available, rating
FROM limit_topn_test
WHERE is_available = true
ORDER BY rating DESC, title ASC
LIMIT 7;

-- Test LIMIT with multiple numeric fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT rating, price
FROM limit_topn_test
WHERE rating > 3.0 AND price < 500
ORDER BY price DESC
LIMIT 12;

SELECT rating, price
FROM limit_topn_test
WHERE rating > 3.0 AND price < 500
ORDER BY price DESC
LIMIT 12;

-- Test LIMIT with complex where clause on mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, category, rating, price
FROM limit_topn_test
WHERE (rating BETWEEN 2.5 AND 4.5) AND category @@@ 'Toys OR Clothing'
ORDER BY price DESC
LIMIT 8;

SELECT title, category, rating, price
FROM limit_topn_test
WHERE (rating BETWEEN 2.5 AND 4.5) AND category @@@ 'Toys OR Clothing'
ORDER BY price DESC
LIMIT 8;

-- Verify actual results of LIMIT queries (not just execution path)
SELECT title, rating, price, category
FROM limit_topn_test
WHERE title @@@ 'Product'
ORDER BY rating DESC
LIMIT 5;

-- Clean up
DROP INDEX IF EXISTS limit_topn_idx;
DROP TABLE IF EXISTS limit_topn_test;

\i common/mixedff_advanced_cleanup.sql

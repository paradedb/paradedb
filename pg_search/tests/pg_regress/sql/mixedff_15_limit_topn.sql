-- Test that LIMIT clause uses TopN execution with mixed fast fields
-- This test ensures that when a LIMIT clause is used with mixed fast fields,
-- the execution uses the optimized TopN execution path

-- Create test table with mixed field types
DROP TABLE IF EXISTS limit_topn_test;
CREATE TABLE limit_topn_test (
    id SERIAL PRIMARY KEY,
    title TEXT,
    description TEXT,
    rating FLOAT,
    price NUMERIC,
    category TEXT,
    is_available BOOLEAN,
    tags TEXT[],
    created_at TIMESTAMP
);

-- Insert test data
INSERT INTO limit_topn_test (title, description, rating, price, category, is_available, created_at)
SELECT
    'Product ' || i,
    'Description for product ' || i,
    (random() * 5)::float,
    (random() * 1000)::numeric,
    (ARRAY['Electronics', 'Books', 'Clothing', 'Food', 'Toys'])[1 + (i % 5)],
    i % 2 = 0,
    NOW() - (i || ' days')::interval
FROM generate_series(1, 100) i;

-- Create search index with multiple fast fields
DROP INDEX IF EXISTS limit_topn_idx;
CREATE INDEX limit_topn_idx ON limit_topn_test
USING columnstore (title, rating, price, category, is_available)
WITH (type='hnsw');

-- Enable execution method tracing to verify TopN usage
SET pg_search.explain_analyze_verbose TO TRUE;

-- Test basic LIMIT with mixed fields (should use TopN)
EXPLAIN ANALYZE
SELECT title, rating, price, category
FROM limit_topn_test
WHERE title ILIKE 'Product%'
ORDER BY rating DESC
LIMIT 10;

-- Test LIMIT with mixed text and numeric fields
EXPLAIN ANALYZE
SELECT title, category, rating, price
FROM limit_topn_test
WHERE category = 'Electronics'
ORDER BY price ASC
LIMIT 5;

-- Test LIMIT with multiple string fields
EXPLAIN ANALYZE
SELECT title, category
FROM limit_topn_test
WHERE category IN ('Books', 'Electronics')
ORDER BY title
LIMIT 15;

-- Test LIMIT with boolean field
EXPLAIN ANALYZE
SELECT title, is_available, rating
FROM limit_topn_test
WHERE is_available = true
ORDER BY rating DESC
LIMIT 7;

-- Test LIMIT with multiple numeric fields
EXPLAIN ANALYZE
SELECT rating, price
FROM limit_topn_test
WHERE rating > 3.0 AND price < 500
ORDER BY price DESC
LIMIT 12;

-- Test LIMIT with complex where clause on mixed fields
EXPLAIN ANALYZE
SELECT title, category, rating, price
FROM limit_topn_test
WHERE (rating BETWEEN 2.5 AND 4.5) AND (category = 'Electronics' OR category = 'Toys')
ORDER BY price DESC
LIMIT 8;

-- Verify actual results of LIMIT queries (not just execution path)
SELECT title, rating, price, category
FROM limit_topn_test
WHERE title ILIKE 'Product%'
ORDER BY rating DESC
LIMIT 5;

-- Clean up
DROP INDEX IF EXISTS limit_topn_idx;
DROP TABLE IF EXISTS limit_topn_test; 

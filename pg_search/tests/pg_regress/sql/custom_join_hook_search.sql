-- Test search integration in custom joins
-- This test focuses on validating search predicate extraction and execution

-- Create the extension first
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable the custom join feature
SET paradedb.enable_custom_join = true;

-- Create test tables with more realistic data
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT
);

CREATE TABLE reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    reviewer_name TEXT,
    review_text TEXT,
    rating INTEGER
);

-- Insert test data with search-friendly content
INSERT INTO products (name, description, category) VALUES 
    ('Laptop Pro', 'High-performance laptop with advanced features for professionals', 'Electronics'),
    ('Wireless Mouse', 'Ergonomic wireless mouse with precision tracking', 'Electronics'),
    ('Coffee Maker', 'Automatic coffee maker with programmable settings', 'Kitchen'),
    ('Running Shoes', 'Lightweight running shoes with excellent cushioning', 'Sports'),
    ('Smartphone X', 'Latest smartphone with cutting-edge technology', 'Electronics');

INSERT INTO reviews (product_id, reviewer_name, review_text, rating) VALUES 
    (1, 'John Doe', 'Excellent laptop with amazing performance and battery life', 5),
    (1, 'Jane Smith', 'Great for professional work, highly recommended', 4),
    (2, 'Bob Wilson', 'Perfect wireless mouse, very responsive and comfortable', 5),
    (3, 'Alice Brown', 'Makes excellent coffee every morning, love the programmable features', 4),
    (4, 'Charlie Davis', 'Best running shoes I have ever owned, great cushioning', 5),
    (5, 'Eva Martinez', 'Amazing smartphone with incredible camera and performance', 5);

-- Create BM25 indexes
CREATE INDEX products_search_idx ON products USING bm25 (
    id,
    name,
    description,
    category
) WITH (
    key_field = 'id',
    text_fields = '{"name": {"tokenizer": {"type": "default"}}, "description": {"tokenizer": {"type": "default"}}, "category": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX reviews_search_idx ON reviews USING bm25 (
    id,
    product_id,
    reviewer_name,
    review_text,
    rating
) WITH (
    key_field = 'id',
    numeric_fields = '{"product_id": {"fast": true}, "rating": {"fast": true}}',
    text_fields = '{"reviewer_name": {"tokenizer": {"type": "default"}}, "review_text": {"tokenizer": {"type": "default"}}}'
);

-- Test 1: Bilateral search join (both sides have search predicates)
-- This should trigger our custom join with real search predicates
SELECT p.name, r.reviewer_name, r.review_text
FROM products p
JOIN reviews r ON p.id = r.product_id
WHERE p.description @@@ 'performance' AND r.review_text @@@ 'excellent';

-- Test 2: Single-sided search join (only one side has search predicate)
SELECT p.name, r.reviewer_name, r.rating
FROM products p
JOIN reviews r ON p.id = r.product_id
WHERE p.category @@@ 'Electronics';

-- Test 3: Complex search with multiple terms
SELECT p.name, p.description, r.review_text
FROM products p
JOIN reviews r ON p.id = r.product_id
WHERE p.description @@@ 'wireless OR smartphone' AND r.review_text @@@ 'amazing OR perfect';

-- Test 4: Search with numeric conditions (should still trigger custom join)
SELECT p.name, r.review_text, r.rating
FROM products p
JOIN reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop' AND r.rating >= 4;

-- Test 5: No search predicates (should use regular join)
SELECT p.name, r.reviewer_name
FROM products p
JOIN reviews r ON p.id = r.product_id
WHERE p.id = 1;

-- Cleanup
DROP TABLE products CASCADE;
DROP TABLE reviews CASCADE;
RESET paradedb.enable_custom_join; 

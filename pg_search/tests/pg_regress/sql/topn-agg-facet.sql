-- Test TopN + Aggregates + Faceting
-- Phase 1: Basic TopN tests with window aggregate detection

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS products CASCADE;

-- Setup test data
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT,
    brand TEXT,
    price NUMERIC,
    rating NUMERIC,
    in_stock BOOLEAN,
    sales INTEGER
);

-- Insert test data
INSERT INTO products (name, description, category, brand, price, rating, in_stock, sales) VALUES
    ('MacBook Pro', 'High-performance laptop for professionals', 'Laptops', 'Apple', 2499, 4.8, true, 150),
    ('Dell XPS 13', 'Compact and powerful ultrabook', 'Laptops', 'Dell', 1299, 4.6, true, 200),
    ('ThinkPad X1', 'Business laptop with great keyboard', 'Laptops', 'Lenovo', 1599, 4.5, true, 180),
    ('HP Spectre', 'Stylish convertible laptop', 'Laptops', 'HP', 1399, 4.4, true, 120),
    ('ASUS ROG', 'Gaming laptop with RTX graphics', 'Laptops', 'ASUS', 1899, 4.7, true, 90);

-- Create BM25 index
CREATE INDEX products_idx ON products
USING bm25(id, name, description, category, brand, price, rating, in_stock, sales)
WITH (
    key_field='id',
    text_fields='{
        "name": {},
        "description": {},
        "brand": {"fast": true}
    }',
    numeric_fields='{
        "price": {"fast": true},
        "rating": {"fast": true},
        "sales": {"fast": true}
    }',
    boolean_fields='{
        "in_stock": {"fast": true}
    }'
);

-- Test 1: Basic TopN without window aggregates
\echo 'Test 1: Basic TopN query'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    category,
    rating
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    category,
    rating
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Test 2: TopN with COUNT(*) OVER () - should trigger window aggregate detection
\echo 'Test 2: TopN with COUNT(*) OVER () (window aggregate detection)'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

SELECT 
    id,
    name,
    rating,
    COUNT(*) OVER () as total_count
FROM products
WHERE description @@@ 'laptop'
ORDER BY rating DESC
LIMIT 3;

-- Cleanup
DROP TABLE products CASCADE;
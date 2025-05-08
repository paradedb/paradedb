-- Test multi-index search with mixed fast fields
-- This test verifies that queries using multiple indices with mixed fast fields work correctly

-- Create main test tables
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS categories;
DROP TABLE IF EXISTS reviews;

CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    price NUMERIC(10,2),
    stock_count INTEGER,
    weight FLOAT,
    is_available BOOLEAN,
    created_at TIMESTAMP
);

CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    product_count INTEGER,
    is_active BOOLEAN
);

CREATE TABLE reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER REFERENCES products(id),
    reviewer_name TEXT,
    content TEXT,
    rating INTEGER,
    helpful_votes INTEGER,
    created_at TIMESTAMP
);

-- Insert test data
INSERT INTO products (name, description, price, stock_count, weight, is_available, created_at)
SELECT
    'Product ' || i,
    'Description for product ' || i || '. This product has various features and specifications.',
    (50.00 + (random() * 950))::numeric(10,2),
    (random() * 200)::integer,
    (0.1 + random() * 20)::float,
    random() < 0.8,  -- 80% are available
    now() - (random() * 365 * 2)::integer * interval '1 day'
FROM generate_series(1, 100) i;

INSERT INTO categories (name, description, product_count, is_active)
VALUES
    ('Electronics', 'Electronic devices and accessories', 40, true),
    ('Books', 'Books and publications', 30, true),
    ('Clothing', 'Apparel and fashion items', 25, true),
    ('Home & Kitchen', 'Home goods and kitchen items', 20, true),
    ('Toys', 'Toys and games', 15, true),
    ('Sports', 'Sporting goods and equipment', 10, true),
    ('Beauty', 'Beauty and personal care items', 5, false),
    ('Automotive', 'Car parts and accessories', 8, true),
    ('Office', 'Office supplies and equipment', 12, true),
    ('Outdoors', 'Outdoor equipment and accessories', 18, true);

-- Insert reviews (10 reviews per product for first 20 products)
INSERT INTO reviews (product_id, reviewer_name, content, rating, helpful_votes, created_at)
SELECT
    (i % 20) + 1,  -- product_id 1-20
    'Reviewer ' || ((i % 50) + 1),  -- 50 different reviewers
    CASE (i % 5)
        WHEN 0 THEN 'Great product, very satisfied with my purchase!'
        WHEN 1 THEN 'Good quality but a bit expensive.'
        WHEN 2 THEN 'Average product, meets basic expectations.'
        WHEN 3 THEN 'Not very impressed, could be better.'
        WHEN 4 THEN 'Terrible product, complete waste of money!'
    END,
    (i % 5) + 1,  -- rating 1-5
    (random() * 50)::integer,  -- helpful votes
    now() - (random() * 180)::integer * interval '1 day'  -- last 6 months
FROM generate_series(1, 200) i;

-- Create join table between products and categories (many-to-many)
DROP TABLE IF EXISTS product_categories;
CREATE TABLE product_categories (
    product_id INTEGER REFERENCES products(id),
    category_id INTEGER REFERENCES categories(id),
    PRIMARY KEY (product_id, category_id)
);

-- Assign each product to 1-3 categories
INSERT INTO product_categories (product_id, category_id)
SELECT
    p.id,
    c.id
FROM products p
CROSS JOIN LATERAL (
    SELECT id FROM categories
    ORDER BY random()
    LIMIT floor(random() * 3) + 1
) c;

-- Create search indices with mixed fast fields
DROP INDEX IF EXISTS products_idx;
DROP INDEX IF EXISTS categories_idx;
DROP INDEX IF EXISTS reviews_idx;

CREATE INDEX products_idx ON products
USING columnstore (name, description, price, stock_count, is_available)
WITH (type='hnsw');

CREATE INDEX categories_idx ON categories
USING columnstore (name, description, product_count, is_active)
WITH (type='hnsw');

CREATE INDEX reviews_idx ON reviews
USING columnstore (reviewer_name, content, rating, helpful_votes)
WITH (type='hnsw');

-- Enable execution method tracing
SET pg_search.explain_analyze_verbose TO TRUE;

-- Test 1: Join between products and categories with search
EXPLAIN ANALYZE
SELECT p.name, p.price, c.name as category
FROM products p
JOIN product_categories pc ON p.id = pc.product_id
JOIN categories c ON pc.category_id = c.id
WHERE p.name ILIKE '%product%' AND c.is_active = true
ORDER BY p.price DESC
LIMIT 10;

-- Test 2: Join between products and reviews with search
EXPLAIN ANALYZE
SELECT p.name, r.rating, r.content
FROM products p
JOIN reviews r ON p.id = r.product_id
WHERE p.description ILIKE '%product%' AND r.rating >= 4
ORDER BY r.helpful_votes DESC
LIMIT 5;

-- Test 3: Three-way join with mixed field conditions
EXPLAIN ANALYZE
SELECT p.name, c.name as category, AVG(r.rating) as avg_rating
FROM products p
JOIN product_categories pc ON p.id = pc.product_id
JOIN categories c ON pc.category_id = c.id
JOIN reviews r ON p.id = r.product_id
WHERE p.price < 500 AND c.product_count > 10
GROUP BY p.name, c.name
HAVING AVG(r.rating) > 3
ORDER BY avg_rating DESC;

-- Test 4: Complex query with multiple indices and mixed fields
EXPLAIN ANALYZE
WITH top_products AS (
    SELECT p.id, p.name, p.price, p.stock_count
    FROM products p
    WHERE p.price BETWEEN 100 AND 800
      AND p.is_available = true
    ORDER BY p.price DESC
    LIMIT 50
),
product_ratings AS (
    SELECT r.product_id, AVG(r.rating) as avg_rating, COUNT(*) as review_count
    FROM reviews r
    WHERE r.rating >= 3
    GROUP BY r.product_id
    HAVING COUNT(*) >= 2
)
SELECT tp.name, tp.price, pr.avg_rating, c.name as category
FROM top_products tp
JOIN product_ratings pr ON tp.id = pr.product_id
JOIN product_categories pc ON tp.id = pc.product_id
JOIN categories c ON pc.category_id = c.id
WHERE c.is_active = true
ORDER BY pr.avg_rating DESC, tp.price DESC;

-- Test 5: Union of results from different tables
EXPLAIN ANALYZE
SELECT 'Product' as type, name as item_name, description as content
FROM products
WHERE name ILIKE '%10%' OR description ILIKE '%feature%'
UNION ALL
SELECT 'Category' as type, name as item_name, description as content
FROM categories
WHERE name ILIKE '%e%'
UNION ALL
SELECT 'Review' as type, reviewer_name as item_name, content
FROM reviews
WHERE content ILIKE '%great%'
ORDER BY type, item_name;

-- Test 6: Subquery with both numeric and text field filtering
EXPLAIN ANALYZE
SELECT p.name, p.price, p.stock_count
FROM products p
WHERE p.id IN (
    SELECT pc.product_id
    FROM product_categories pc
    JOIN categories c ON pc.category_id = c.id
    WHERE c.name ILIKE '%electronics%' OR c.name ILIKE '%clothing%'
)
AND p.stock_count > 50
AND p.price < 500
ORDER BY p.price;

-- Test 7: Join with conditional logic and mixed fields
EXPLAIN ANALYZE
SELECT 
    p.name,
    p.price,
    CASE 
        WHEN r.rating IS NULL THEN 'No reviews'
        WHEN r.rating < 3 THEN 'Poor reviews'
        WHEN r.rating < 4 THEN 'Average reviews'
        ELSE 'Great reviews'
    END as review_status
FROM products p
LEFT JOIN (
    SELECT product_id, AVG(rating) as rating
    FROM reviews
    GROUP BY product_id
) r ON p.id = r.product_id
WHERE p.is_available = true
  AND p.price BETWEEN 200 AND 600
ORDER BY 
    CASE 
        WHEN r.rating IS NULL THEN 0
        ELSE r.rating
    END DESC,
    p.price;

-- Test 8: Multi-index intersection
EXPLAIN ANALYZE
SELECT p.name, p.price, r.content, r.rating
FROM products p
JOIN reviews r ON p.id = r.product_id
JOIN product_categories pc ON p.id = pc.product_id
JOIN categories c ON pc.category_id = c.id
WHERE p.name ILIKE '%product%'
  AND r.rating > 3
  AND c.name = 'Electronics'
  AND p.is_available = true
ORDER BY r.rating DESC, p.price DESC;

-- Verify actual results of multi-index search
SELECT p.name, p.price, c.name as category
FROM products p
JOIN product_categories pc ON p.id = pc.product_id
JOIN categories c ON pc.category_id = c.id
WHERE p.name ILIKE '%product 1%' AND c.is_active = true
ORDER BY p.price DESC
LIMIT 5;

-- Clean up
DROP INDEX IF EXISTS products_idx;
DROP INDEX IF EXISTS categories_idx;
DROP INDEX IF EXISTS reviews_idx;
DROP TABLE IF EXISTS product_categories;
DROP TABLE IF EXISTS reviews;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS categories; 

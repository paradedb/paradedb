CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan = ON;

-- Create filter_agg_test test table
DROP TABLE IF EXISTS filter_agg_test CASCADE;
CREATE TABLE filter_agg_test (
    id INT PRIMARY KEY,
    title TEXT,
    description TEXT,
    category TEXT,
    brand TEXT,
    status TEXT,
    price NUMERIC,
    rating INTEGER,
    in_stock BOOLEAN,
    views INTEGER
);

-- Insert deterministic test data covering all scenarios
INSERT INTO filter_agg_test (id, title, description, category, brand, status, price, rating, in_stock, views) VALUES
-- Electronics (Apple)
(1, 'MacBook Pro', 'laptop computer with keyboard', 'electronics', 'Apple', 'available', 2499.99, 5, true, 1500),
(2, 'iMac Desktop', 'desktop computer with monitor', 'electronics', 'Apple', 'available', 1999.99, 5, true, 1200),
(3, 'iPad Tablet', 'tablet with stylus', 'electronics', 'Apple', 'sold', 899.99, 4, false, 2000),
-- Electronics (Samsung)
(4, 'Galaxy Laptop', 'laptop computer gaming', 'electronics', 'Samsung', 'available', 1799.99, 4, true, 800),
(5, 'Samsung Monitor', 'monitor ultra wide', 'electronics', 'Samsung', 'available', 599.99, 4, true, 600),
(6, 'Galaxy Tablet', 'tablet android device', 'electronics', 'Samsung', 'sold', 649.99, 3, false, 900),
-- Electronics (Generic)
(7, 'Gaming Keyboard', 'keyboard mechanical gaming', 'electronics', 'Generic', 'available', 149.99, 3, true, 400),
(8, 'Wireless Mouse', 'mouse wireless pro', 'electronics', 'Generic', 'available', 79.99, 4, true, 300),
-- Clothing
(9, 'Developer T-Shirt', 'shirt for programming', 'clothing', 'TechWear', 'available', 24.99, 4, true, 200),
(10, 'Database Hoodie', 'hoodie with logo', 'clothing', 'TechWear', 'available', 59.99, 5, true, 350),
(11, 'Running Shoes', 'shoes for running', 'clothing', 'SportsBrand', 'sold', 129.99, 4, false, 180),
(12, 'Casual Jeans', 'jeans casual wear', 'clothing', 'FashionCo', 'available', 79.99, 3, true, 120),
-- Books
(13, 'Database Systems', 'database design book', 'books', 'TechPress', 'available', 49.99, 5, true, 1800),
(14, 'Search Engines', 'search engine design', 'books', 'TechPress', 'available', 59.99, 5, true, 1600),
(15, 'SQL Performance', 'sql optimization guide', 'books', 'DataBooks', 'sold', 39.99, 4, false, 1400),
(16, 'PostgreSQL Guide', 'postgresql advanced topics', 'books', 'DataBooks', 'available', 44.99, 4, true, 1200),
-- Sports
(17, 'Tennis Racket', 'racket for tennis', 'sports', 'SportsCorp', 'available', 199.99, 4, true, 250),
(18, 'Basketball', 'basketball official size', 'sports', 'SportsCorp', 'available', 29.99, 3, true, 150),
(19, 'Soccer Ball', 'soccer ball professional', 'sports', 'PlayTime', 'sold', 39.99, 4, false, 200),
(20, 'Golf Clubs', 'golf club set premium', 'sports', 'GolfPro', 'available', 899.99, 5, true, 100);

-- Create BM25 index with fast fields for all aggregation scenarios
CREATE INDEX filter_agg_idx ON filter_agg_test
USING bm25(id, title, description, category, brand, status, price, rating, in_stock, views)
WITH (
    key_field='id',
    text_fields='{
        "title": {},
        "description": {},
        "category": {"fast": true},
        "brand": {"fast": true},
        "status": {"fast": true}
    }',
    numeric_fields='{
        "price": {"fast": true},
        "rating": {"fast": true},
        "views": {"fast": true}
    }',
    boolean_fields='{
        "in_stock": {"fast": true}
    }'
);

-- Test for issue where querying views with BM25 indexes using @@@ operator
-- would fail with "Cannot open relation with oid=Oid(0)"
-- This test ensures that BM25 searches work properly on views that combine
-- multiple tables with BM25 indexes using UNION ALL

CREATE EXTENSION IF NOT EXISTS pg_search;

-- Create two tables with identical schema
CREATE TABLE products_2023 (
    id SERIAL,
    product_name TEXT,
    amount DECIMAL(18,2),
    sale_date DATE
);

CREATE TABLE products_2024 (
    id SERIAL,
    product_name TEXT,
    amount DECIMAL(18,2),
    sale_date DATE
);

-- Insert test data
INSERT INTO products_2023 (product_name, amount, sale_date) VALUES
('Laptop', 1200.00, '2023-01-15'),
('Desktop Computer', 800.00, '2023-02-20'),
('Gaming Mouse', 50.00, '2023-03-10');

INSERT INTO products_2024 (product_name, amount, sale_date) VALUES
('Tablet', 500.00, '2024-01-05'),
('Smartphone', 700.00, '2024-02-15'),
('Wireless Headphones', 150.00, '2024-03-25');

-- Create BM25 indexes on both tables
CREATE INDEX idx_products_2023_bm25 ON products_2023
USING bm25 (id, product_name, amount, sale_date)
WITH (
    key_field = 'id',
    text_fields = '{"product_name": {}}'
);

CREATE INDEX idx_products_2024_bm25 ON products_2024
USING bm25 (id, product_name, amount, sale_date)
WITH (
    key_field = 'id',
    text_fields = '{"product_name": {}}'
);

-- Create view that combines both tables
CREATE VIEW products_view AS
SELECT * FROM products_2023
UNION ALL
SELECT * FROM products_2024;

-- Test individual table searches work correctly
SELECT id, product_name FROM products_2023 WHERE product_name @@@ 'laptop' ORDER BY id;

SELECT id, product_name FROM products_2024 WHERE product_name @@@ 'tablet' ORDER BY id;

-- Test that the view query works without the @@@ operator
SELECT id, product_name FROM products_view WHERE product_name LIKE '%Laptop%' ORDER BY id;

-- The main test: This should work without throwing "Cannot open relation with oid=Oid(0)"
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, product_name FROM products_view WHERE product_name @@@ 'laptop OR tablet' ORDER BY id;

SELECT id, product_name FROM products_view WHERE product_name @@@ 'laptop OR tablet' ORDER BY id;

-- Test with more complex query
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, product_name, amount FROM products_view 
WHERE product_name @@@ 'laptop OR tablet OR computer' 
AND amount > 100 
ORDER BY amount DESC;

SELECT id, product_name, amount FROM products_view 
WHERE product_name @@@ 'laptop OR tablet OR computer' 
AND amount > 100 
ORDER BY amount DESC;

-- Cleanup
DROP VIEW products_view;
DROP TABLE products_2023;
DROP TABLE products_2024;

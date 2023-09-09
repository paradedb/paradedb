-- Basic search query
SELECT *
FROM products
WHERE products @@@ 'description:keyboard OR category:electronics OR rating>2';
-- With BM25 scoring
SELECT paradedb.score_bm25(ctid), * 
FROM products 
WHERE products @@@ 'category:electronics OR description:keyboard';
-- Test real-time search
INSERT INTO products (description, rating, category) VALUES ('New keyboard', 5, 'Electronics');
DELETE FROM products WHERE id = 1;
UPDATE products SET description = 'PVC Keyboard' WHERE id = 2;
SELECT *
FROM products
WHERE products @@@ 'description:keyboard OR category:electronics OR rating>2';

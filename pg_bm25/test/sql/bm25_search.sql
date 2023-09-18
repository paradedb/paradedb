-- this is needed to ensure consistency of printouts with postgres versions older than 12. Can be
-- deleted if we drop support for postgres 11.
ALTER SYSTEM SET extra_float_digits TO 0;
select pg_reload_conf();


-- Basic search query
SELECT *
FROM products
WHERE products @@@ 'description:keyboard OR category:electronics OR rating>2';
-- With BM25 scoring
SELECT paradedb.rank_bm25(ctid), * 
FROM products 
WHERE products @@@ 'category:electronics OR description:keyboard';
-- Test real-time search
INSERT INTO products (description, rating, category) VALUES ('New keyboard', 5, 'Electronics');
DELETE FROM products WHERE id = 1;
UPDATE products SET description = 'PVC Keyboard' WHERE id = 2;
SELECT *
FROM products
WHERE products @@@ 'description:keyboard OR category:electronics OR rating>2';
-- Test search in another namespace/schema
SELECT *
FROM paradedb.mock_items
WHERE mock_items @@@ 'description:keyboard';

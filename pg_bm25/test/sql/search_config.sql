-- Basic seach query
SELECT id, description, rating, category FROM products WHERE products @@@ 'category:electronics';
-- With trailing delimiter
SELECT id, description, rating, category FROM products WHERE products @@@ 'category:electronics:::';
-- With limit
SELECT id, description, rating, category FROM products WHERE products @@@ 'category:electronics:::limit=2';
-- With limit and trailing &
SELECT id, description, rating, category FROM products WHERE products @@@ 'category:electronics:::limit=2&';
-- With limit and offset
SELECT id, description, rating, category FROM products WHERE products @@@ 'category:electronics:::limit=2&offset=1';



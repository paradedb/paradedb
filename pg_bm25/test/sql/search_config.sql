-- Basic seach query
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electronics';
-- With trailing delimiter
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electronics:::';
-- With limit
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electronics:::limit=2';
-- With limit and trailing &
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electronics:::limit=2&';
-- With limit and offset
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electronics:::limit=2&offset=1';
-- With fuzzy field
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electornics:::fuzzy_fields=category';
-- Without fuzzy field
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electornics';

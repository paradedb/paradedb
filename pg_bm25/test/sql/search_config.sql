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
-- With fuzzy field and transpose_cost_one=false and distance=1
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'keybaord:::fuzzy_fields=description&transpose_cost_one=false&distance=1';
-- With fuzzy field and transpose_cost_one=true and distance=1
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'keybaord:::fuzzy_fields=description&transpose_cost_one=true&distance=1';
-- With fuzzy and regex field
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'com:::regex_fields=description&fuzzy_fields=description';
-- With regex field 
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'com:::regex_fields=description';

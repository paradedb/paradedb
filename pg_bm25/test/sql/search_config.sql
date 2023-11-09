-- Basic seach query
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electronics';
-- With trailing delimiter
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electronics:::';
-- With limit
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electronics' LIMIT 2;
-- With limit and offset
SELECT id, description, rating, category FROM search_config WHERE search_config @@@ 'category:electronics:::offset=1' LIMIT 2;
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
-- Default highlighting without max_num_chars
SELECT description, rating, category, paradedb.highlight_bm25(ctid, 'idxsearchconfig', 'description') FROM search_config WHERE search_config @@@ 'description:keyboard OR category:electronics' ORDER BY paradedb.rank_bm25(ctid) DESC LIMIT 5;
-- max_num_chars is set to 14 
SELECT description, rating, category, paradedb.highlight_bm25(ctid, 'idxsearchconfig', 'description') FROM search_config WHERE search_config @@@ 'description:keyboard OR category:electronics:::max_num_chars=14' ORDER BY paradedb.rank_bm25(ctid) DESC LIMIT 5;

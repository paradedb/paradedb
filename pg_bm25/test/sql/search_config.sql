-- Basic seach query
SELECT id, description, rating, category FROM search_config.search('category:electronics') ORDER BY id;
-- With limit
SELECT id, description, rating, category FROM search_config.search('category:electronics', limit_rows => 2) ORDER BY id;
-- With limit and offset
SELECT id, description, rating, category FROM search_config.search('category:electronics', limit_rows => 2, offset_rows => 1) ORDER BY id;
-- With fuzzy field
SELECT id, description, rating, category FROM search_config.search('category:electornics', fuzzy_fields => 'category') ORDER BY id;
-- Without fuzzy field
SELECT id, description, rating, category FROM search_config.search('category:electornics') ORDER BY id;
-- With fuzzy field and transpose_cost_one=false and distance=1
SELECT id, description, rating, category FROM search_config.search('description:keybaord', fuzzy_fields => 'description', transpose_cost_one => false, distance => 1) ORDER BY id;
-- With fuzzy field and transpose_cost_one=true and distance=1
SELECT id, description, rating, category FROM search_config.search('description:keybaord', fuzzy_fields => 'description', transpose_cost_one => true, distance => 1) ORDER BY id;
-- With regex field 
SELECT id, description, rating, category FROM search_config.search('com', regex_fields => 'description') ORDER BY id;
-- Default highlighting without max_num_chars
SELECT s.id, description, rating, category, highlight_bm25 FROM search_config.search('description:keyboard OR category:electronics') as s LEFT JOIN search_config.highlight('description:keyboard OR category:electronics', highlight_field => 'description') as h ON s.id = H.id LEFT JOIN search_config.rank('description:keyboard OR category:electronics') as r ON s.id = r.id ORDER BY s.id DESC LIMIT 5;
-- max_num_chars is set to 14 
SELECT s.id, description, rating, category, highlight_bm25 FROM search_config.search('description:keyboard OR category:electronics', max_num_chars => 14) as s LEFT JOIN search_config.highlight('description:keyboard OR category:electronics', highlight_field => 'description', max_num_chars => 14) as h ON s.id = H.id LEFT JOIN search_config.rank('description:keyboard OR category:electronics', max_num_chars => 14) as r ON s.id = r.id ORDER BY s.id DESC LIMIT 5;

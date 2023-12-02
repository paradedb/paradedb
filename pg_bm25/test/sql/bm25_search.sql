-- Basic search query
SELECT *
FROM bm25_search
WHERE bm25_search @@@ 'description:keyboard OR category:electronics';

-- With BM25 scoring
SELECT paradedb.rank_bm25(bm25_search.id), * 
FROM bm25_search 
WHERE bm25_search @@@ 'category:electronics OR description:keyboard';

-- Test JSON search 
SELECT *
FROM bm25_search
WHERE bm25_search @@@ 'metadata.color:white';

-- Test real-time search
INSERT INTO bm25_search (description, rating, category) VALUES ('New keyboard', 5, 'Electronics');
DELETE FROM bm25_search WHERE id = 1;
UPDATE bm25_search SET description = 'PVC Keyboard' WHERE id = 2;
SELECT *
FROM bm25_search
WHERE bm25_search @@@ 'description:keyboard OR category:electronics';

-- Test search in another namespace/schema
SELECT *
FROM paradedb.bm25_test_table
WHERE bm25_test_table @@@ 'description:keyboard';

-- Test search with default tokenizer: no results
SELECT *
FROM paradedb.bm25_test_table
WHERE bm25_test_table @@@ 'description:earbud';

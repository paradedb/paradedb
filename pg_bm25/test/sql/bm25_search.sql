-- this is needed to ensure consistency of printouts with postgres versions older than 12. Can be
-- deleted if we drop support for postgres 11.
ALTER SYSTEM SET extra_float_digits TO 0;
select pg_reload_conf();


-- Basic search query
SELECT *
FROM bm25_search
WHERE bm25_search @@@ 'description:keyboard OR category:electronics OR rating>2';

-- With BM25 scoring
SELECT paradedb.rank_bm25(ctid), * 
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
WHERE bm25_search @@@ 'description:keyboard OR category:electronics OR rating>2';

-- Test search in another namespace/schema
SELECT *
FROM paradedb.mock_items
WHERE mock_items @@@ 'description:keyboard';

-- Test search with default tokenizer: no results
SELECT *
FROM paradedb.mock_items
WHERE mock_items @@@ 'description:earbud';


-- With BM25 scoring
SELECT r.rank_bm25, s.* FROM bm25_search.search('category:electronics OR description:keyboard') as s
LEFT JOIN bm25_search.rank('category:electronics OR description:keyboard') as r ON s.id = r.id;

-- Test JSON search 
SELECT * FROM bm25_search.search('metadata.color:white');

-- Test real-time search
INSERT INTO bm25_search (description, rating, category) VALUES ('New keyboard', 5, 'Electronics');
DELETE FROM bm25_search WHERE id = 1;
UPDATE bm25_search SET description = 'PVC Keyboard' WHERE id = 2;
SELECT * FROM bm25_search.search('description:keyboard OR category:electronics');

-- Test search with default tokenizer: no results
SELECT * FROM bm25_search.search('description:earbud');

-- Test sequential scan syntax
SELECT * FROM paradedb.bm25_test_table
WHERE paradedb.search_tantivy(
    paradedb.bm25_test_table.*,
    jsonb_build_object(
        'index_name', 'bm25_search_bm25_index',
        'table_name', 'bm25_test_table',
        'schema_name', 'paradedb',
        'key_field', 'id',
        'query', 'category:electronics'
    )
);

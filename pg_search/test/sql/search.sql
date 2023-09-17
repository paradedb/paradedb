CREATE INDEX idx_mock_items ON mock_items USING bm25 ((mock_items.*));
CREATE INDEX ON mock_items USING hnsw (embedding vector_l2_ops);

WITH query AS (
    SELECT 
        ctid,
        paradedb.l2_normalized_bm25(ctid, 'idx_mock_items', 'keyboard') as bm25,
        ('[1,2,3]' <-> embedding) / paradedb.l2_norm('[1,2,3]' <-> embedding) OVER () as hnsw
    FROM
        mock_items 
)
SELECT 
    mock_items.description,
    mock_items.category,
    mock_items.rating,
    paradedb.weighted_mean(query.bm25, query.hnsw, ARRAY[0.8, 0.2]) as score_hybrid 
FROM mock_items
JOIN query ON mock_items.ctid = query.ctid
ORDER BY score_hybrid DESC;

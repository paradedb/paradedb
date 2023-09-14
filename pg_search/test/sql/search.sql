CREATE INDEX idx_mock_items ON mock_items USING bm25 ((mock_items.*));
CREATE INDEX ON mock_items USING hnsw (embedding vector_l2_ops);

SELECT 
    description,
    category,
    rating,
    paradedb.weighted_mean(
        paradedb.score_bm25(ctid, 'idx_mock_items', 'description:keyboard'),
        '[1,2,3]' <-> embedding,
        ARRAY[0.8, 0.2]
    ) AS score_hybrid
FROM mock_items
ORDER BY score_hybrid DESC;

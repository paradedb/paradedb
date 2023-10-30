SELECT description FROM mock_items ORDER BY sparse_embedding <==> '[0,0,0,3,0,2,0,4,0,1]' LIMIT 5;
CREATE INDEX idx_mock_items ON mock_items USING sparse_hnsw(sparse_embedding);
SET enable_seqscan = off;
SELECT description FROM mock_items ORDER BY sparse_embedding <==> '[0,0,0,3,0,2,0,4,0,1]' LIMIT 5;
DROP INDEX idx_mock_items;
CREATE INDEX idx_mock_items ON mock_items USING sparse_hnsw(sparse_embedding) WITH (ef_search=64, ef_construction=64, m=16);
SELECT description FROM mock_items ORDER BY sparse_embedding <==> '[0,0,0,3,0,2,0,4,0,1]' LIMIT 5;


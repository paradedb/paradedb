-- Test vector search with pgvector integration

CREATE EXTENSION IF NOT EXISTS vector;

-- Create table with vector column
CREATE TABLE test_vectors (
    id SERIAL PRIMARY KEY,
    description TEXT,
    embedding vector(3)
);

-- Insert test data
INSERT INTO test_vectors (description, embedding) VALUES
    ('red apple', '[1, 0, 0]'),
    ('green apple', '[0, 1, 0]'),
    ('blue sky', '[0, 0, 1]'),
    ('red sky', '[0.9, 0, 0.1]'),
    ('green grass', '[0.1, 0.9, 0]');

-- Create BM25 index that includes the vector column
CREATE INDEX idx_test_vectors ON test_vectors USING bm25 (id, description, embedding)
WITH (key_field = 'id');

-- Hybrid search: text filter + vector distance ordering
SELECT id, description FROM test_vectors
WHERE description @@@ 'apple'
ORDER BY embedding <-> '[1, 0, 0]'
LIMIT 3;

-- Hybrid search: different query
SELECT id, description FROM test_vectors
WHERE description @@@ 'sky'
ORDER BY embedding <-> '[0, 0, 1]'
LIMIT 3;

-- Clean up
DROP INDEX idx_test_vectors;
DROP TABLE test_vectors;

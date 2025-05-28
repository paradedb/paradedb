-- Test performance and edge cases for custom join execution
-- This test validates various scenarios and edge cases

-- Create the extension first
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable the custom join feature
SET paradedb.enable_custom_join = true;

-- Create test tables for performance testing
CREATE TABLE documents_perf (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    author TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE tags_perf (
    id SERIAL PRIMARY KEY,
    document_id INTEGER,
    tag_name TEXT,
    tag_value TEXT
);

-- Insert larger dataset for performance testing
INSERT INTO documents_perf (title, content, author) 
SELECT 
    'Document ' || i,
    'This is document number ' || i || ' with content about ' || 
    CASE (i % 5) 
        WHEN 0 THEN 'technology and innovation'
        WHEN 1 THEN 'science and research'
        WHEN 2 THEN 'business and finance'
        WHEN 3 THEN 'health and medicine'
        ELSE 'education and learning'
    END,
    'Author ' || ((i % 10) + 1)
FROM generate_series(1, 50) i;

INSERT INTO tags_perf (document_id, tag_name, tag_value)
SELECT 
    (i % 50) + 1,
    CASE (i % 4)
        WHEN 0 THEN 'category'
        WHEN 1 THEN 'priority'
        WHEN 2 THEN 'status'
        ELSE 'type'
    END,
    CASE (i % 3)
        WHEN 0 THEN 'high'
        WHEN 1 THEN 'medium'
        ELSE 'low'
    END
FROM generate_series(1, 100) i;

-- Create BM25 indexes
CREATE INDEX documents_perf_idx ON documents_perf USING bm25 (
    id,
    title,
    content,
    author
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}, "author": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX tags_perf_idx ON tags_perf USING bm25 (
    id,
    document_id,
    tag_name,
    tag_value
) WITH (
    key_field = 'id',
    numeric_fields = '{"document_id": {"fast": true}}',
    text_fields = '{"tag_name": {"tokenizer": {"type": "default"}}, "tag_value": {"tokenizer": {"type": "default"}}}'
);

-- Test 1: Large result set join
SELECT COUNT(*) as total_matches
FROM documents_perf d
JOIN tags_perf t ON d.id = t.document_id
WHERE d.content @@@ 'technology OR science' AND t.tag_value @@@ 'high OR medium';

-- Test 2: Selective search (should return fewer results)
SELECT d.title, t.tag_name, t.tag_value
FROM documents_perf d
JOIN tags_perf t ON d.id = t.document_id
WHERE d.content @@@ 'innovation' AND t.tag_value @@@ 'high'
LIMIT 5;

-- Test 3: Cross-product scenario (many-to-many)
SELECT COUNT(*) as cross_product_count
FROM documents_perf d
JOIN tags_perf t ON d.id = t.document_id
WHERE d.author @@@ 'Author' AND t.tag_name @@@ 'category OR priority';

-- Test 4: Empty result set
SELECT d.title, t.tag_value
FROM documents_perf d
JOIN tags_perf t ON d.id = t.document_id
WHERE d.content @@@ 'nonexistent_term' AND t.tag_value @@@ 'impossible_value';

-- Test 5: Single result
SELECT d.title, d.author, t.tag_name
FROM documents_perf d
JOIN tags_perf t ON d.id = t.document_id
WHERE d.title @@@ 'Document 1' AND t.tag_value @@@ 'high'
LIMIT 1;

-- Test 6: EXPLAIN to see execution plan
EXPLAIN (COSTS OFF, BUFFERS OFF) 
SELECT d.title, t.tag_value
FROM documents_perf d
JOIN tags_perf t ON d.id = t.document_id
WHERE d.content @@@ 'technology' AND t.tag_value @@@ 'high';

-- Cleanup
DROP TABLE documents_perf CASCADE;
DROP TABLE tags_perf CASCADE;
RESET paradedb.enable_custom_join; 

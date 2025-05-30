-- Simple TopN Join Test
-- Test that TopN join optimization works for basic bilateral search queries

\set ON_ERROR_STOP on

CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable join optimization
SET paradedb.enable_custom_join = true;
SET paradedb.enable_topn_join_optimization = true;

-- Create simple test tables
DROP TABLE IF EXISTS simple_files CASCADE;
DROP TABLE IF EXISTS simple_docs CASCADE;

CREATE TABLE simple_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE simple_files (
    id SERIAL PRIMARY KEY,
    filename TEXT,
    content TEXT,
    doc_id INTEGER REFERENCES simple_docs(id)
);

-- Insert test data
INSERT INTO simple_docs (title, content) VALUES 
    ('Doc 1', 'database postgresql advanced system'),
    ('Doc 2', 'machine learning data analysis'),
    ('Doc 3', 'web development javascript');

INSERT INTO simple_files (filename, content, doc_id) VALUES 
    ('readme.txt', 'database tutorial advanced guide', 1),
    ('analysis.py', 'data processing machine learning', 2),
    ('app.js', 'web application javascript code', 3);

-- Create BM25 indexes
CREATE INDEX simple_docs_idx ON simple_docs
USING bm25 ("id",
    "content",
    "title"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "content": {
            "tokenizer": {"type": "default"}
        },
        "title": {
            "tokenizer": {"type": "default"}
        }
    }'
);
CREATE INDEX simple_files_idx ON simple_files
USING bm25 ("id",
    "content",
    "filename",
    "doc_id"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "content": {
            "tokenizer": {"type": "default"}
        },
        "filename": {
            "tokenizer": {"type": "default"}
        }
    }',
    numeric_fields = '{
        "doc_id": {}
    }'
);


-- Test 1: Basic TopN join with LIMIT (should trigger TopN optimization)
\echo 'Test 1: Basic TopN join with LIMIT'
SELECT d.title, f.filename
FROM simple_docs d
JOIN simple_files f ON d.id = f.doc_id
WHERE d.content @@@ 'database'
  AND f.content @@@ 'database'
ORDER BY d.id
LIMIT 2;

-- Test 2: EXPLAIN to check if TopN optimization is used
\echo 'Test 2: EXPLAIN for TopN join'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT d.title, f.filename
FROM simple_docs d
JOIN simple_files f ON d.id = f.doc_id
WHERE d.content @@@ 'database'
  AND f.content @@@ 'advanced'
ORDER BY d.id
LIMIT 1;

-- Cleanup
DROP TABLE simple_files CASCADE;
DROP TABLE simple_docs CASCADE; 

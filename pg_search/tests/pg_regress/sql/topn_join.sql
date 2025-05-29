-- TopN Join Regression Tests
-- Test the TopN join optimization for queries with LIMIT and ORDER BY

\set ON_ERROR_STOP on

-- Create the extension first
CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_custom_join = true;
SET paradedb.enable_topn_join_optimization = true;
SET paradedb.enable_join_debug_logging = true;

DROP TABLE IF EXISTS topn_files;
DROP TABLE IF EXISTS topn_documents;
DROP TABLE IF EXISTS topn_categories;

-- Create test tables
CREATE TABLE topn_documents (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    category_id INTEGER
);

CREATE TABLE topn_files (
    id SERIAL PRIMARY KEY,
    filename TEXT NOT NULL,
    content TEXT NOT NULL,
    document_id INTEGER REFERENCES topn_documents(id)
);

CREATE TABLE topn_categories (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT
);

-- Insert test data
INSERT INTO topn_categories (name, description) VALUES 
    ('Tech', 'Technology articles'),
    ('Science', 'Scientific papers'),
    ('News', 'News articles');

INSERT INTO topn_documents (title, content, category_id) VALUES 
    ('Database Systems', 'PostgreSQL is a powerful database system with advanced search capabilities', 1),
    ('Machine Learning', 'Artificial intelligence and machine learning algorithms for data analysis', 1),
    ('Climate Change', 'Research on global climate patterns and environmental impact studies', 2),
    ('Space Exploration', 'Mars rover missions and deep space telescope discoveries', 2),
    ('Market Updates', 'Stock market analysis and economic trends for Q4 2024', 3),
    ('Tech News', 'Latest developments in cloud computing and containerization technologies', 3);

INSERT INTO topn_files (filename, content, document_id) VALUES 
    ('db_intro.pdf', 'Introduction to database design principles and PostgreSQL features', 1),
    ('db_advanced.pdf', 'Advanced database optimization techniques and performance tuning', 1),
    ('ml_basics.txt', 'Machine learning fundamentals and neural network architectures', 2),
    ('ml_deep.txt', 'Deep learning models for natural language processing applications', 2),
    ('climate_data.csv', 'Temperature and precipitation data from weather stations worldwide', 3),
    ('climate_analysis.py', 'Python scripts for analyzing climate change patterns and trends', 3),
    ('mars_photos.jpg', 'High resolution images from Mars rover exploration missions', 4),
    ('telescope_data.fits', 'Astronomical observations from the James Webb Space Telescope', 4),
    ('market_report.xlsx', 'Quarterly financial analysis and market performance metrics', 5),
    ('economics.pdf', 'Economic indicators and inflation trends analysis report', 5),
    ('cloud_guide.md', 'Best practices for cloud infrastructure and container orchestration', 6),
    ('tech_trends.json', 'Technology adoption metrics and industry forecast data', 6);

-- Create BM25 indexes
CREATE INDEX topn_documents_idx ON topn_documents
USING bm25 ("id",
    "content",
    "title",
    "category_id"
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
    }',
    numeric_fields = '{
        "category_id": {}
    }'
);

CREATE INDEX topn_files_idx ON topn_files
USING bm25 ("id",
    "content",
    "filename",
    "document_id"
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
        "document_id": {}
    }'
);

CREATE INDEX topn_categories_idx ON topn_categories
USING bm25 ("id",
    "name",
    "description"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "name": {
            "tokenizer": {"type": "default"}
        },
        "description": {
            "tokenizer": {"type": "default"}
        }
    }'
);

-- Test 1: Basic TopN join with LIMIT and score-based ordering
\echo 'Test 1: Basic TopN join with LIMIT and score ordering'
SELECT 
    d.title,
    f.filename,
    paradedb.score(d.id) as doc_score,
    paradedb.score(f.id) as file_score,
    (paradedb.score(d.id) + paradedb.score(f.id)) as combined_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'database OR PostgreSQL'
  AND f.content @@@ 'database OR advanced'
ORDER BY combined_score DESC
LIMIT 3;

-- Test 2: TopN join with different LIMIT values
\echo 'Test 2: TopN join with LIMIT 1'
SELECT 
    d.title,
    f.filename,
    (paradedb.score(d.id) + paradedb.score(f.id)) as combined_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'machine learning'
  AND f.content @@@ 'machine OR learning'
ORDER BY combined_score DESC
LIMIT 1;

-- Test 3: TopN join with ASC ordering
\echo 'Test 3: TopN join with ASC ordering'
SELECT 
    d.title,
    f.filename,
    (paradedb.score(d.id) + paradedb.score(f.id)) as combined_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'climate OR science'
  AND f.content @@@ 'data OR analysis'
ORDER BY combined_score ASC
LIMIT 2;

-- Test 4: TopN join that should find no matches
\echo 'Test 4: TopN join with no matches'
SELECT 
    d.title,
    f.filename,
    (paradedb.score(d.id) + paradedb.score(f.id)) as combined_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'nonexistent_term_xyz'
  AND f.content @@@ 'another_nonexistent_term_abc'
ORDER BY combined_score DESC
LIMIT 5;

-- Test 5: TopN join with larger LIMIT to test expansion logic
\echo 'Test 5: TopN join with larger LIMIT'
SELECT 
    d.title,
    f.filename,
    (paradedb.score(d.id) + paradedb.score(f.id)) as combined_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'data OR analysis OR system'
  AND f.content @@@ 'data OR analysis OR advanced'
ORDER BY combined_score DESC
LIMIT 10;

-- Test 6: Disable TopN optimization and compare
\echo 'Test 6: Same query with TopN optimization disabled'
SET paradedb.enable_topn_join_optimization = false;

SELECT 
    d.title,
    f.filename,
    (paradedb.score(d.id) + paradedb.score(f.id)) as combined_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'database OR PostgreSQL'
  AND f.content @@@ 'database OR advanced'
ORDER BY combined_score DESC
LIMIT 3;

-- Re-enable for remaining tests
SET paradedb.enable_topn_join_optimization = true;

-- Test 7: TopN join with additional WHERE conditions
\echo 'Test 7: TopN join with additional WHERE conditions'
SELECT 
    d.title,
    f.filename,
    d.category_id,
    (paradedb.score(d.id) + paradedb.score(f.id)) as combined_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'technology OR computing'
  AND f.content @@@ 'technology OR guide'
  AND d.category_id IN (1, 3)
ORDER BY combined_score DESC
LIMIT 4;

-- Test 8: Test EXPLAIN to see if TopN optimization is being used
\echo 'Test 8: EXPLAIN for TopN join query'
EXPLAIN (ANALYZE false, BUFFERS false, COSTS false, TIMING false, SUMMARY false)
SELECT 
    d.title,
    f.filename,
    (paradedb.score(d.id) + paradedb.score(f.id)) as combined_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'database'
  AND f.content @@@ 'advanced'
ORDER BY combined_score DESC
LIMIT 2;

-- Test 9: TopN join without ORDER BY (should still optimize with LIMIT)
\echo 'Test 9: TopN join with LIMIT but no explicit ORDER BY'
SELECT 
    d.title,
    f.filename,
    (paradedb.score(d.id) + paradedb.score(f.id)) as combined_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'machine OR artificial'
  AND f.content @@@ 'learning OR neural'
LIMIT 2;

-- Test 10: TopN join that wouldn't qualify (unilateral search)
\echo 'Test 10: Unilateral search join (should not use TopN optimization)'
SELECT 
    d.title,
    f.filename,
    paradedb.score(d.id) as doc_score
FROM topn_documents d
JOIN topn_files f ON d.id = f.document_id
WHERE d.content @@@ 'space OR exploration'
ORDER BY doc_score DESC
LIMIT 3;

-- Clean up
DROP TABLE topn_files;
DROP TABLE topn_documents;
DROP TABLE topn_categories; 

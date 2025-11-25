-- LEFT JOIN LATERAL TopN optimization tests
-- Tests the ability to use TopN executor for LEFT JOIN LATERAL queries
-- where the left side drives the query execution

-- Load the pg_search extension
DROP EXTENSION IF EXISTS pg_search CASCADE;
CREATE EXTENSION pg_search;

-- Disable parallel workers for consistent test results
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS articles CASCADE;
DROP TABLE IF EXISTS comments CASCADE;
DROP TABLE IF EXISTS authors CASCADE;
DROP TABLE IF EXISTS tags CASCADE;

-- Create test tables
CREATE TABLE articles (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    author_id INT,
    created_at TIMESTAMP,
    score_value FLOAT
);

CREATE TABLE comments (
    id SERIAL PRIMARY KEY,
    article_id INT,
    content TEXT,
    author_name TEXT,
    created_at TIMESTAMP,
    rating INT
);

CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT,
    bio TEXT,
    expertise TEXT
);

CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    article_id INT,
    tag_name TEXT
);

-- Insert test data
INSERT INTO authors (name, bio, expertise) VALUES
    ('John Doe', 'Tech writer specializing in databases', 'database'),
    ('Jane Smith', 'AI researcher and author', 'artificial intelligence'),
    ('Bob Johnson', 'Cloud computing expert', 'cloud computing'),
    ('Alice Brown', 'Security specialist', 'cybersecurity'),
    ('Charlie Wilson', 'Full stack developer', 'web development');

INSERT INTO articles (title, content, author_id, created_at, score_value) VALUES
    ('Understanding PostgreSQL', 'PostgreSQL is a powerful database system with advanced features', 1, '2024-01-01'::TIMESTAMP, 75.5),
    ('Machine Learning Basics', 'Introduction to machine learning concepts and algorithms', 2, '2024-01-02'::TIMESTAMP, 82.3),
    ('Cloud Native Applications', 'Building applications for the cloud using modern patterns', 3, '2024-01-03'::TIMESTAMP, 68.9),
    ('Database Security Best Practices', 'How to secure your database systems effectively', 4, '2024-01-04'::TIMESTAMP, 91.2),
    ('Web Development in 2024', 'Modern web development tools and frameworks', 5, '2024-01-05'::TIMESTAMP, 55.7),
    ('Advanced SQL Techniques', 'Complex queries and optimization strategies for SQL', 1, '2024-01-06'::TIMESTAMP, 88.4),
    ('Neural Networks Explained', 'Deep dive into neural network architectures', 2, '2024-01-07'::TIMESTAMP, 79.1),
    ('Kubernetes Deployment', 'Deploying applications on Kubernetes clusters', 3, '2024-01-08'::TIMESTAMP, 62.8),
    ('Zero Trust Security', 'Implementing zero trust architecture in organizations', 4, '2024-01-09'::TIMESTAMP, 94.5),
    ('React vs Vue Comparison', 'Comparing popular frontend frameworks', 5, '2024-01-10'::TIMESTAMP, 71.3),
    ('Computer Vision Applications', 'Real world applications of computer vision technology', 2, '2024-01-11'::TIMESTAMP, 86.6),
    ('Database Indexing Strategies', 'How to optimize database performance with indexes', 1, '2024-01-12'::TIMESTAMP, 77.9),
    ('Serverless Architecture', 'Building serverless applications on AWS and Azure', 3, '2024-01-13'::TIMESTAMP, 65.2),
    ('Encryption Fundamentals', 'Understanding encryption algorithms and protocols', 4, '2024-01-14'::TIMESTAMP, 89.8),
    ('GraphQL API Design', 'Designing efficient GraphQL APIs for modern applications', 5, '2024-01-15'::TIMESTAMP, 58.4);

-- Insert comments for articles with deterministic timestamps and ratings
INSERT INTO comments (article_id, content, author_name, created_at, rating) 
SELECT 
    (generate_series % 15) + 1,
    'Comment ' || generate_series || ' about article',
    'Commenter ' || (generate_series % 20),
    '2024-01-01'::TIMESTAMP + (generate_series || ' hours')::INTERVAL,
    (generate_series % 5) + 1
FROM generate_series(1, 100) AS generate_series;

-- Insert tags
INSERT INTO tags (article_id, tag_name) VALUES
    (1, 'database'), (1, 'postgresql'), (1, 'technology'),
    (2, 'AI'), (2, 'machine-learning'), (2, 'technology'),
    (3, 'cloud'), (3, 'devops'), (3, 'technology'),
    (4, 'security'), (4, 'database'), (4, 'technology'),
    (5, 'web'), (5, 'frontend'), (5, 'technology'),
    (6, 'database'), (6, 'sql'), (6, 'technology'),
    (7, 'AI'), (7, 'deep-learning'), (7, 'technology'),
    (8, 'kubernetes'), (8, 'devops'), (8, 'technology'),
    (9, 'security'), (9, 'architecture'), (9, 'technology'),
    (10, 'javascript'), (10, 'frontend'), (10, 'technology'),
    (11, 'computer-vision'), (11, 'AI'), (11, 'technology'),
    (12, 'database'), (12, 'performance'), (12, 'technology'),
    (13, 'serverless'), (13, 'cloud'), (13, 'technology'),
    (14, 'encryption'), (14, 'security'), (14, 'technology'),
    (15, 'graphql'), (15, 'api'), (15, 'technology');

-- Create BM25 indexes with fast field for sorting
CREATE INDEX articles_bm25_idx ON articles USING bm25 (id, title, content, created_at) WITH (
    key_field = 'id',
    text_fields = '{
        "title": { "fast": true, "tokenizer": {"type": "default"} },
        "content": { "tokenizer": {"type": "default"} }
    }',
    datetime_fields = '{"created_at": {"fast": true}}'
);
CREATE INDEX comments_bm25_idx ON comments USING bm25 (id, content) WITH (key_field = 'id');
CREATE INDEX authors_bm25_idx ON authors USING bm25 (id, name, bio, expertise) WITH (key_field = 'id');

-- =============================================================================
-- TEST 1: Basic LEFT JOIN LATERAL with TopN optimization (should use TopN)
-- =============================================================================
-- Test that a simple LEFT JOIN LATERAL with LIMIT uses TopN executor
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    latest_comment.comment_id,
    latest_comment.comment_content
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        c.id as comment_id,
        c.content as comment_content
    FROM comments c
    WHERE c.article_id = a.id
    ORDER BY c.created_at DESC
    LIMIT 1
) latest_comment ON true
WHERE a.content @@@ 'database'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- Execute the query to verify results
SELECT 
    a.id,
    a.title,
    latest_comment.comment_id,
    latest_comment.comment_content
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        c.id as comment_id,
        c.content as comment_content
    FROM comments c
    WHERE c.article_id = a.id
    ORDER BY c.created_at DESC
    LIMIT 1
) latest_comment ON true
WHERE a.content @@@ 'database'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- =============================================================================
-- TEST 2: LEFT JOIN LATERAL without LIMIT (should NOT use TopN)
-- =============================================================================
-- EXPLAIN to verify Normal scan (not TopN) due to missing LIMIT
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    latest_comment.comment_id
FROM articles a
LEFT JOIN LATERAL (
    SELECT c.id as comment_id
    FROM comments c
    WHERE c.article_id = a.id
    ORDER BY c.created_at DESC
    LIMIT 1
) latest_comment ON true
WHERE a.content @@@ 'technology'
ORDER BY paradedb.score(a.id) DESC;

-- Execute to verify results work without LIMIT
SELECT 
    a.id,
    a.title,
    paradedb.score(a.id) as score,
    latest_comment.comment_id
FROM articles a
LEFT JOIN LATERAL (
    SELECT c.id as comment_id
    FROM comments c
    WHERE c.article_id = a.id
    ORDER BY c.created_at DESC
    LIMIT 1
) latest_comment ON true
WHERE a.content @@@ 'technology'
ORDER BY paradedb.score(a.id) DESC;

-- =============================================================================
-- TEST 3: LEFT JOIN LATERAL with WHERE clause referencing both tables
-- =============================================================================
-- This should NOT use TopN because WHERE references the right table
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    c.comment_count
FROM articles a
LEFT JOIN LATERAL (
    SELECT COUNT(*) as comment_count
    FROM comments c
    WHERE c.article_id = a.id
) c ON true
WHERE a.content @@@ 'database' 
  AND c.comment_count > 5
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- Execute to verify results
SELECT 
    a.id,
    a.title,
    paradedb.score(a.id) as score,
    c.comment_count
FROM articles a
LEFT JOIN LATERAL (
    SELECT COUNT(*) as comment_count
    FROM comments c
    WHERE c.article_id = a.id
) c ON true
WHERE a.content @@@ 'database' 
  AND c.comment_count > 5
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- =============================================================================
-- TEST 4: Regular LEFT JOIN (not LATERAL) with LIMIT
-- =============================================================================
-- This should NOT use TopN optimization (not a LATERAL join)
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    au.name as author_name
FROM articles a
LEFT JOIN authors au ON a.author_id = au.id
WHERE a.content @@@ 'technology'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- Execute to verify results
SELECT 
    a.id,
    a.title,
    paradedb.score(a.id) as score,
    au.name as author_name
FROM articles a
LEFT JOIN authors au ON a.author_id = au.id
WHERE a.content @@@ 'technology'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- =============================================================================
-- TEST 5: LEFT JOIN LATERAL with multiple aggregations
-- =============================================================================
-- Complex LATERAL subquery with aggregations, should still use TopN if conditions met
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    stats.total_comments,
    stats.avg_rating,
    stats.latest_comment_date
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        COUNT(*) as total_comments,
        AVG(rating) as avg_rating,
        MAX(created_at) as latest_comment_date
    FROM comments c
    WHERE c.article_id = a.id
) stats ON true
WHERE a.content @@@ 'machine learning'
ORDER BY paradedb.score(a.id) DESC
LIMIT 10;

-- Execute to verify results
SELECT 
    a.id,
    a.title,
    stats.total_comments,
    stats.avg_rating
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        COUNT(*) as total_comments,
        AVG(rating)::NUMERIC(10,2) as avg_rating
    FROM comments c
    WHERE c.article_id = a.id
) stats ON true
WHERE a.content @@@ 'machine learning'
ORDER BY paradedb.score(a.id) DESC
LIMIT 10;

-- =============================================================================
-- TEST 6: LEFT JOIN LATERAL with ORDER BY on indexed fast field (not score)
-- =============================================================================
-- Should use TopN with ORDER BY on an indexed field marked as fast
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    a.created_at,
    latest.comment_time
FROM articles a
LEFT JOIN LATERAL (
    SELECT MAX(created_at) as comment_time
    FROM comments c
    WHERE c.article_id = a.id
) latest ON true
WHERE a.content @@@ 'cloud'
ORDER BY a.created_at DESC
LIMIT 5;

-- Execute to verify results
SELECT 
    a.id,
    a.title,
    a.created_at,
    latest.comment_time
FROM articles a
LEFT JOIN LATERAL (
    SELECT MAX(created_at) as comment_time
    FROM comments c
    WHERE c.article_id = a.id
) latest ON true
WHERE a.content @@@ 'cloud'
ORDER BY a.created_at DESC
LIMIT 5;

-- =============================================================================
-- TEST 7: Multiple LEFT JOIN LATERAL
-- =============================================================================
-- Multiple LATERAL joins, left-side driven
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    comment_stats.comment_count,
    tag_list.tags
FROM articles a
LEFT JOIN LATERAL (
    SELECT COUNT(*) as comment_count
    FROM comments c
    WHERE c.article_id = a.id
) comment_stats ON true
LEFT JOIN LATERAL (
    SELECT string_agg(tag_name, ', ') as tags
    FROM tags t
    WHERE t.article_id = a.id
) tag_list ON true
WHERE a.content @@@ 'database security'
ORDER BY paradedb.score(a.id) DESC
LIMIT 3;

-- Execute to verify
SELECT 
    a.id,
    a.title,
    comment_stats.comment_count,
    tag_list.tags
FROM articles a
LEFT JOIN LATERAL (
    SELECT COUNT(*) as comment_count
    FROM comments c
    WHERE c.article_id = a.id
) comment_stats ON true
LEFT JOIN LATERAL (
    SELECT string_agg(tag_name, ', ') as tags
    FROM tags t
    WHERE t.article_id = a.id
) tag_list ON true
WHERE a.content @@@ 'database security'
ORDER BY paradedb.score(a.id) DESC
LIMIT 3;

-- =============================================================================
-- TEST 8: LEFT JOIN LATERAL with complex WHERE clause (only left table)
-- =============================================================================
-- Complex WHERE but still only references left table - should use TopN
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    a.score_value,
    recent_activity.last_comment
FROM articles a
LEFT JOIN LATERAL (
    SELECT MAX(created_at) as last_comment
    FROM comments c
    WHERE c.article_id = a.id
) recent_activity ON true
WHERE a.content @@@ 'machine learning' 
  AND a.author_id IN (1, 2)
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- Execute to verify results
SELECT 
    a.id,
    a.title,
    ROUND(a.score_value::numeric, 2) as score_value,
    paradedb.score(a.id) as score,
    recent_activity.last_comment
FROM articles a
LEFT JOIN LATERAL (
    SELECT MAX(created_at) as last_comment
    FROM comments c
    WHERE c.article_id = a.id
) recent_activity ON true
WHERE a.content @@@ 'machine learning' 
  AND a.author_id IN (1, 2)
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- =============================================================================
-- TEST 9: LEFT JOIN LATERAL with NULL handling
-- =============================================================================
-- Verify NULL handling for articles without comments
-- EXPLAIN to verify execution plan
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    comment_info.has_comments,
    comment_info.comment_count
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        TRUE as has_comments,
        COUNT(*) as comment_count
    FROM comments c
    WHERE c.article_id = a.id
    HAVING COUNT(*) > 0
) comment_info ON true
WHERE a.content @@@ 'encryption'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- Execute to verify NULL handling
SELECT 
    a.id,
    a.title,
    comment_info.has_comments,
    comment_info.comment_count
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        TRUE as has_comments,
        COUNT(*) as comment_count
    FROM comments c
    WHERE c.article_id = a.id
    HAVING COUNT(*) > 0
) comment_info ON true
WHERE a.content @@@ 'encryption'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- =============================================================================
-- TEST 10: Verify TopN with score() in SELECT list
-- =============================================================================
-- Ensure score projection works correctly with LEFT JOIN LATERAL
-- EXPLAIN to verify execution plan
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    paradedb.score(a.id) as relevance_score,
    stats.comment_count
FROM articles a
LEFT JOIN LATERAL (
    SELECT COUNT(*) as comment_count
    FROM comments c
    WHERE c.article_id = a.id
) stats ON true
WHERE a.content @@@ 'technology'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- Execute to verify score projection
SELECT 
    a.id,
    a.title,
    paradedb.score(a.id) as relevance_score,
    stats.comment_count
FROM articles a
LEFT JOIN LATERAL (
    SELECT COUNT(*) as comment_count
    FROM comments c
    WHERE c.article_id = a.id
) stats ON true
WHERE a.content @@@ 'technology'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- =============================================================================
-- TEST 11: Nested JOIN with LATERAL (exercises nested join detection)
-- =============================================================================
-- With the relaxed logic, this now uses TopN optimization because:
-- 1. There's a LEFT JOIN in the query
-- 2. There are LATERAL references (even if not to 'articles')
-- 3. WHERE/ORDER BY only reference the left table ('articles')
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    au.name as author_name,
    comment_stats.comment_count
FROM articles a
LEFT JOIN (
    authors au
    INNER JOIN LATERAL (
        SELECT 
            COUNT(*) as comment_count
        FROM comments c
        WHERE c.article_id IN (
            SELECT id FROM articles WHERE author_id = au.id
        )
    ) comment_stats ON true
) ON a.author_id = au.id
WHERE a.content @@@ 'database'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- Execute to verify results
SELECT 
    a.id,
    a.title,
    au.name as author_name,
    comment_stats.comment_count
FROM articles a
LEFT JOIN (
    authors au
    INNER JOIN LATERAL (
        SELECT 
            COUNT(*) as comment_count
        FROM comments c
        WHERE c.article_id IN (
            SELECT id FROM articles WHERE author_id = au.id
        )
    ) comment_stats ON true
) ON a.author_id = au.id
WHERE a.content @@@ 'database'
ORDER BY paradedb.score(a.id) DESC
LIMIT 5;

-- =============================================================================
-- TEST 12: Deep nested joins with multiple LATERAL references
-- =============================================================================
-- With the relaxed logic, this uses TopN optimization even though the LATERAL
-- subquery doesn't directly reference 'articles'. TopN applies when:
-- 1. There's any LEFT JOIN + LATERAL in the query
-- 2. WHERE/ORDER BY only reference the left table
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    complex.name as author_name,
    complex.total_activity
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        au.id,
        au.name,
        article_stats.article_count + COALESCE(comment_stats.comment_count, 0) as total_activity
    FROM authors au
    INNER JOIN LATERAL (
        SELECT COUNT(*) as article_count
        FROM articles
        WHERE author_id = au.id
    ) article_stats ON true
    LEFT JOIN LATERAL (
        SELECT COUNT(*) as comment_count
        FROM comments c
        WHERE c.author_name = au.name
    ) comment_stats ON true
) complex ON a.author_id = complex.id
WHERE a.content @@@ 'database'
ORDER BY paradedb.score(a.id) DESC
LIMIT 3;

-- Execute to verify the complex nested structure works
SELECT 
    a.id,
    a.title,
    complex.name as author_name,
    complex.total_activity
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        au.id,
        au.name,
        article_stats.article_count + COALESCE(comment_stats.comment_count, 0) as total_activity
    FROM authors au
    INNER JOIN LATERAL (
        SELECT COUNT(*) as article_count
        FROM articles
        WHERE author_id = au.id
    ) article_stats ON true
    LEFT JOIN LATERAL (
        SELECT COUNT(*) as comment_count
        FROM comments c
        WHERE c.author_name = au.name
    ) comment_stats ON true
) complex ON a.author_id = complex.id
WHERE a.content @@@ 'database'
ORDER BY paradedb.score(a.id) DESC
LIMIT 3;

-- =============================================================================
-- TEST 13: LEFT JOIN LATERAL with paradedb.snippet function
-- =============================================================================
-- Demonstrates TopN optimization with snippet generation for matched content
EXPLAIN (COSTS OFF)
SELECT 
    a.id,
    a.title,
    paradedb.snippet(a.content, '<b>', '</b>') as content_snippet,
    latest_comment.comment_content
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        c.content as comment_content
    FROM comments c
    WHERE c.article_id = a.id
    ORDER BY c.created_at DESC
    LIMIT 1
) latest_comment ON true
WHERE a.content @@@ 'database'
ORDER BY paradedb.score(a.id) DESC
LIMIT 3;

-- Execute to verify snippet generation works with LEFT JOIN LATERAL
SELECT 
    a.id,
    a.title,
    paradedb.snippet(a.content, '<b>', '</b>') as content_snippet,
    latest_comment.comment_content
FROM articles a
LEFT JOIN LATERAL (
    SELECT 
        c.content as comment_content
    FROM comments c
    WHERE c.article_id = a.id
    ORDER BY c.created_at DESC
    LIMIT 1
) latest_comment ON true
WHERE a.content @@@ 'database'
ORDER BY paradedb.score(a.id) DESC
LIMIT 3;

-- =============================================================================
-- CLEANUP
-- =============================================================================
DROP TABLE IF EXISTS articles CASCADE;
DROP TABLE IF EXISTS comments CASCADE;
DROP TABLE IF EXISTS authors CASCADE;
DROP TABLE IF EXISTS tags CASCADE;

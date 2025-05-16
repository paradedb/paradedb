-- Tests score function behavior with mixed fast fields

\i common/mixedff_advanced_setup.sql

\echo 'Test: Score function behavior'

-- Create test table with mixed field types
DROP TABLE IF EXISTS score_test;
CREATE TABLE score_test (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    author TEXT,
    rating INTEGER,
    views FLOAT,
    published_date DATE,
    is_featured BOOLEAN
);

-- Insert test data with deterministic values
INSERT INTO score_test (title, content, author, rating, views, published_date, is_featured)
SELECT
    'Post ' || i,
    'This is content for post ' || i || '. It contains some searchable text and keywords like technology, science, research, and development.',
    'Author ' || (1 + (i % 5)),
    (1 + (i % 5)),
    (100 * i)::float,  -- Deterministic view counts
    '1988-04-29'::date + (i % 365) * '1 day'::interval,  -- Deterministic dates
    i % 7 = 0  -- Deterministic featured pattern
FROM generate_series(1, 100) i;

-- Add some specific posts for testing
INSERT INTO score_test (title, content, author, rating, views, published_date, is_featured)
VALUES
    ('Special Technology Post', 'This post is all about technology and innovative research.', 'Author Expert', 5, 9999, '2023-06-15', true),
    ('Advanced Science Research', 'Detailed explanation of scientific breakthroughs and research methodology.', 'Author Expert', 5, 8888, '2023-07-20', true),
    ('Technology Trends Analysis', 'Analysis of current and future technology trends and developments.', 'Author Expert', 4, 7777, '2023-08-10', true);

-- Create search index with mixed fast fields
DROP INDEX IF EXISTS score_test_idx;
CREATE INDEX score_test_idx ON score_test
USING bm25 (id, title, content, author, rating, views, is_featured)
WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}, "author": {"tokenizer": {"type": "default"}, "fast": true}}',
    numeric_fields = '{"rating": {"fast": true}, "views": {"fast": true}}',
    boolean_fields = '{"is_featured": {"fast": true}}'
);

-- Test 1: Basic score function with text field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, paradedb.score(id), rating
FROM score_test
WHERE content @@@ 'technology'
ORDER BY title, paradedb.score(id), rating DESC
LIMIT 10;

SELECT title, paradedb.score(id), rating
FROM score_test
WHERE content @@@ 'technology'
ORDER BY title, paradedb.score(id), rating DESC
LIMIT 10;

-- Test 2: Score function with mixed field types in selection
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, author, rating, views, paradedb.score(id)
FROM score_test
WHERE content @@@ 'research'
ORDER BY title, author, rating, views, paradedb.score(id) DESC
LIMIT 5;

SELECT title, author, rating, views, paradedb.score(id)
FROM score_test
WHERE content @@@ 'research'
ORDER BY title, author, rating, views, paradedb.score(id) DESC
LIMIT 5;

-- Test 3: Score function with multiple conditions on different field types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, author, paradedb.score(id)
FROM score_test
WHERE content @@@ 'technology' AND rating >= 4 AND is_featured = true
ORDER BY title, author, paradedb.score(id) DESC;

SELECT title, author, paradedb.score(id)
FROM score_test
WHERE content @@@ 'technology' AND rating >= 4 AND is_featured = true
ORDER BY title, author, paradedb.score(id) DESC;

-- Test 4: Using score in a CTE with mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
WITH scored_posts AS (
    SELECT title, author, rating, paradedb.score(id) as relevance
    FROM score_test
    WHERE content @@@ 'science OR research'
)
SELECT title, author, rating, relevance
FROM scored_posts
WHERE rating > 3
ORDER BY title, author, relevance DESC
LIMIT 10;

WITH scored_posts AS (
    SELECT title, author, rating, paradedb.score(id) as relevance
    FROM score_test
    WHERE content @@@ 'science OR research'
)
SELECT title, author, rating, relevance
FROM scored_posts
WHERE rating > 3
ORDER BY title, author, relevance DESC
LIMIT 10;

-- Test 5: Score function in subquery with mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT sp.title, sp.author, sp.relevance
FROM (
    SELECT title, author, paradedb.score(id) as relevance
    FROM score_test
    WHERE content @@@ 'technology' AND rating > 3
) sp
WHERE sp.relevance > 0.5
ORDER BY sp.title, sp.author, sp.relevance DESC;

SELECT sp.title, sp.author, sp.relevance
FROM (
    SELECT title, author, paradedb.score(id) as relevance
    FROM score_test
    WHERE content @@@ 'technology' AND rating > 3
) sp
WHERE sp.relevance > 0.5
ORDER BY sp.title, sp.author, sp.relevance DESC;

-- Test 6: Score function with UNION
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, author, paradedb.score(id) as relevance
FROM score_test
WHERE content @@@ 'technology'
UNION ALL
SELECT title, author, paradedb.score(id) as relevance
FROM score_test
WHERE content @@@ 'science' AND NOT (title @@@ 'technology')
ORDER BY title, author, relevance DESC
LIMIT 10;

SELECT title, author, paradedb.score(id) as relevance
FROM score_test
WHERE content @@@ 'technology'
UNION ALL
SELECT title, author, paradedb.score(id) as relevance
FROM score_test
WHERE content @@@ 'science' AND NOT (title @@@ 'technology')
ORDER BY title, author, relevance DESC
LIMIT 10;

-- Test 7: Score function with JOIN using mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT a.title, a.author, a.rating, a.score, b.title as related_title
FROM (
    SELECT title, author, rating, paradedb.score(id) as score
    FROM score_test
    WHERE content @@@ 'technology'
    ORDER BY score DESC
    LIMIT 5
) a
JOIN (
    SELECT title, author
    FROM score_test
    WHERE author IN (SELECT author FROM score_test WHERE content @@@ 'technology')
) b ON a.author = b.author AND a.title <> b.title
ORDER BY a.title, a.author, a.rating, a.score, b.title;

SELECT a.title, a.author, a.rating, a.score, b.title as related_title
FROM (
    SELECT title, author, rating, paradedb.score(id) as score
    FROM score_test
    WHERE content @@@ 'technology'
    ORDER BY score DESC
    LIMIT 5
) a
JOIN (
    SELECT title, author
    FROM score_test
    WHERE author IN (SELECT author FROM score_test WHERE content @@@ 'technology')
) b ON a.author = b.author AND a.title <> b.title
ORDER BY a.title, a.author, a.rating, a.score, b.title;

-- Test 8: Score function with CASE expression and mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    title, 
    author,
    rating,
    CASE 
        WHEN paradedb.score(id) > 0.8 THEN 'High Relevance'
        WHEN paradedb.score(id) > 0.5 THEN 'Medium Relevance'
        ELSE 'Low Relevance'
    END as relevance_category
FROM score_test
WHERE content @@@ 'research OR development'
ORDER BY title, author, paradedb.score(id) DESC
LIMIT 10;

SELECT 
    title, 
    author,
    rating,
    CASE 
        WHEN paradedb.score(id) > 0.8 THEN 'High Relevance'
        WHEN paradedb.score(id) > 0.5 THEN 'Medium Relevance'
        ELSE 'Low Relevance'
    END as relevance_category
FROM score_test
WHERE content @@@ 'research OR development'
ORDER BY title, author, paradedb.score(id) DESC
LIMIT 10;

-- Verify actual results of score function (not just execution method)
SELECT title, author, rating, paradedb.score(id) as relevance
FROM score_test
WHERE content @@@ 'technology' AND rating >= 4
ORDER BY title, author, relevance DESC
LIMIT 5;

-- Test combination of score function and different fast field types
SELECT title, author, rating, views, paradedb.score(id) as relevance
FROM score_test
WHERE content @@@ 'research' AND rating > 3 AND views > 1000
ORDER BY title, author, relevance DESC
LIMIT 3;

\i common/mixedff_advanced_cleanup.sql

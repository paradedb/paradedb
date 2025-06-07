-- Test case for issue #2530: BM25 scores return null when not all predicates are indexed
-- This test demonstrates the problem where score functions return null/zero results
-- when search predicates are handled by join filters instead of custom scan filters

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Setup test tables
DROP TABLE IF EXISTS authors;
DROP TABLE IF EXISTS books;

CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT,
    age INT
);

CREATE TABLE books (
    id SERIAL PRIMARY KEY,
    author_id INT,
    content TEXT,
    titles TEXT[],
    metadata JSONB
);

-- Insert test data
INSERT INTO authors (name, age) VALUES
('J.K. Rowling', 55),
('Stephen King', 75),
('Agatha Christie', 80),
('Dan Brown', 60),
('J.R.R. Tolkien', 100);

INSERT INTO books (author_id, content, titles, metadata) VALUES
(1, 'This is a test test of the scoring function with multiple test words', ARRAY['test', 'score', 'function'], '{"test": "test"}'),
(2, 'Another test of the scoring scoring function with repeated scoring words', ARRAY['test', 'test', 'function'], '{"test": "test"}'),
(1, 'Yet another test test test of the function function function', ARRAY['test', 'score', 'test'], '{"test": "test"}');

-- Create BM25 indexes
CREATE INDEX ON authors USING bm25 (
    id,
    name,
    age
) WITH (key_field = 'id');

CREATE INDEX ON books USING bm25 (
    id,
    author_id,
    content,
    titles
) WITH (key_field = 'id');

-- Commented out for now because it has an oid in the output, which changes on every run
-- Show the problematic query plan
-- This query causes predicates to be split between scan filters and join filters
-- EXPLAIN (VERBOSE, COSTS OFF) 
-- SELECT
--     b.id as book_id,
--     paradedb.score(a.id) as author_score,
--     paradedb.score(b.id) as book_score
-- FROM books b
-- JOIN authors a ON b.author_id = a.id
-- WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50';

-- Execute the query to show the score issue
-- Currently, this returns null/zero scores for author_score
-- even though 'Rowling' should contribute to the BM25 score calculation
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50';

-- For comparison, show a working case where predicates can be pushed down
-- This should work correctly because all predicates for 'a' can be pushed to the authors scan
SELECT
    a.id as author_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score
FROM authors a
WHERE a.name @@@ 'Rowling' AND a.age @@@ '>50';

-- Show another working case with books
SELECT
    b.id as book_id,
    paradedb.score(b.id) as book_score
FROM books b
WHERE b.content @@@ 'test';

-- Test case with only join predicate - should show the issue more clearly
-- This demonstrates scores being null when the scoring predicate is in the join filter
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE a.name @@@ 'Rowling' AND b.content @@@ 'test';

-- Test with mixed predicates - some indexed, some not
-- This should show partial scores based on what can be indexed
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (a.name @@@ 'King' OR b.content @@@ 'scoring') AND a.age > 70;

SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (a.name @@@ 'King' OR b.content @@@ 'scoring');

SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (a.name @@@ 'King' OR b.content @@@ 'scoring') AND a.age > 60;

SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (a.name @@@ 'King' OR b.content @@@ 'scoring') OR a.age > 60;

-- Test score comparison - direct vs join query
-- Show how the same author gets different scores in different query contexts
-- Direct query (should work)
SELECT 
    'Direct Query' as query_type,
    a.id as author_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score
FROM authors a 
WHERE a.name @@@ 'Rowling';

-- Join query (currently shows issue)
SELECT 
    'Join Query' as query_type,
    a.id as author_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE a.name @@@ 'Rowling';

-- Test with different join types to see if the issue persists
-- LEFT JOIN case
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
LEFT JOIN authors a ON b.author_id = a.id
WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50';

-- RIGHT JOIN case
SELECT
    a.id as author_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    COALESCE(paradedb.score(b.id), 0) as book_score
FROM books b
RIGHT JOIN authors a ON b.author_id = a.id
WHERE (a.name @@@ 'Christie' OR b.content @@@ 'test') AND a.age > 60;

-- Test multiple score functions in same query
-- This tests if score calculation is consistent across multiple score calls
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score_1,
    paradedb.score(a.id) as author_score_2,  -- Should be same as author_score_1
    paradedb.score(b.id) as book_score_1,
    paradedb.score(b.id) as book_score_2     -- Should be same as book_score_1
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (b.content @@@ 'function' OR a.name @@@ 'King') AND a.age @@@ '>50';

-- Test score with ORDER BY to verify scores make sense for ranking
-- Even if scores are null/zero, the ordering should still work
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50'
ORDER BY paradedb.score(a.id) DESC, paradedb.score(b.id) DESC;

-- Test combining scores and snippets to show they should be consistent
-- Both should reflect the same search context
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.score(a.id) as author_score,
    paradedb.snippet(a.name) as author_snippet,
    paradedb.score(b.id) as book_score,
    paradedb.snippet(b.content) as book_snippet
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50';

-- Cleanup
DROP TABLE IF EXISTS books;
DROP TABLE IF EXISTS authors; 

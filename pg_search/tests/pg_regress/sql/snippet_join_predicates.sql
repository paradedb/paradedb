-- Test case for issue #2553: Snippets get lost when not all predicates can be pushed down
-- This test demonstrates the problem where snippet functions return empty results
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
(1, 'This is a test test of the snippet function with multiple test words', ARRAY['test', 'snippet', 'function'], '{"test": "test"}'),
(2, 'Another test of the snippet snippet function with repeated snippet words', ARRAY['test', 'test', 'function'], '{"test": "test"}'),
(1, 'Yet another test test test of the function function function', ARRAY['test', 'snippet', 'test'], '{"test": "test"}');

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
-- -- Show the problematic query plan
-- -- This query causes predicates to be split between scan filters and join filters
-- EXPLAIN (VERBOSE, COSTS OFF) 
-- SELECT
--     b.id as book_id,
--     paradedb.snippet(a.name) as author_snippet,
--     paradedb.snippet_positions(a.name) as author_positions
-- FROM books b
-- JOIN authors a ON b.author_id = a.id
-- WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50';

-- Execute the query to show the snippet issue
-- Currently, this returns empty snippets for author_snippet and author_positions
-- even though 'Rowling' should be highlighted in a.name
SELECT
    b.id as book_id,
    paradedb.snippet(a.name) as author_snippet,
    paradedb.snippet_positions(a.name) as author_positions,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50'
ORDER BY b.id;

-- For comparison, show a working case where predicates can be pushed down
-- This should work correctly because all predicates for 'a' can be pushed to the authors scan
SELECT
    a.id as author_id,
    paradedb.snippet(a.name) as author_snippet,
    paradedb.snippet_positions(a.name) as author_positions,
    paradedb.score(a.id) as author_score
FROM authors a
WHERE a.name @@@ 'Rowling' AND a.age @@@ '>50'
ORDER BY a.id;

-- Show another working case with books
SELECT
    b.id as book_id,
    paradedb.snippet(b.content) as content_snippet,
    paradedb.snippet_positions(b.content) as content_positions,
    paradedb.score(b.id) as book_score
FROM books b
WHERE b.content @@@ 'test'
ORDER BY b.id;

-- Commented out for now because it has an oid in the output, which changes on every run
-- -- Test case with multiple snippet fields in join - should demonstrate the issue more clearly
-- EXPLAIN (VERBOSE, COSTS OFF)
-- SELECT
--     b.id as book_id,
--     a.name as author_name,
--     paradedb.snippet(a.name) as author_snippet,
--     paradedb.snippet(b.content) as content_snippet,
--     paradedb.snippet_positions(a.name) as author_positions,
--     paradedb.snippet_positions(b.content) as content_positions
-- FROM books b
-- JOIN authors a ON b.author_id = a.id
-- WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50';

-- Execute the multi-snippet query
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.snippet(a.name) as author_snippet,
    paradedb.snippet(b.content) as content_snippet,
    paradedb.snippet_positions(a.name) as author_positions,
    paradedb.snippet_positions(b.content) as content_positions,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50'
ORDER BY b.id, a.id;

-- Additional test: Show that score functions work (they use placeholder mechanism)
-- This demonstrates that the target list propagation works, but snippets don't
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.snippet(a.name) as author_snippet,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
JOIN authors a ON b.author_id = a.id
WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50'
ORDER BY b.id, a.id;

-- Test with different join types to see if the issue persists
-- LEFT JOIN case
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.snippet(a.name) as author_snippet,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM books b
LEFT JOIN authors a ON b.author_id = a.id
WHERE (b.content @@@ 'test' OR a.name @@@ 'Rowling') AND a.age @@@ '>50'
ORDER BY b.id, a.id;

-- Cleanup
DROP TABLE IF EXISTS books;
DROP TABLE IF EXISTS authors; 

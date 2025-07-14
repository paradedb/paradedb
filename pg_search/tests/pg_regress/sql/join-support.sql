-- Test cases demonstrating join support with @@@ operator and OR conditions
-- This test showcases what we can support and what we cannot with cross-table OR decomposition

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Setup test tables for join scenarios
DROP TABLE IF EXISTS authors;
DROP TABLE IF EXISTS books;
DROP TABLE IF EXISTS publishers;

CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT,
    bio TEXT,
    birth_year INTEGER
);

CREATE TABLE books (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    author_id INTEGER REFERENCES authors(id),
    publication_year INTEGER,
    price DECIMAL(10,2)
);

CREATE TABLE publishers (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    founded_year INTEGER
);

-- Insert test data
INSERT INTO authors (name, bio, birth_year) VALUES
('John Smith', 'Famous science fiction author', 1960),
('Jane Doe', 'Mystery and thriller writer', 1970),
('Bob Johnson', 'Non-fiction technology author', 1965),
('Alice Brown', 'Romance novelist', 1975);

INSERT INTO books (title, content, author_id, publication_year, price) VALUES
('Future Technologies', 'artificial intelligence machine learning robots', 1, 2020, 29.99),
('The Mystery House', 'detective mystery crime investigation', 2, 2018, 19.99),
('Programming Guide', 'software development coding algorithms', 3, 2019, 39.99),
('Love in Paris', 'romance love story relationships', 4, 2021, 24.99),
('Space Adventure', 'science fiction space exploration aliens', 1, 2022, 34.99);

INSERT INTO publishers (name, description, founded_year) VALUES
('TechBooks Publishing', 'technology and science publications', 1990),
('Mystery House Press', 'crime and mystery novels', 1985),
('Romance World', 'romantic fiction publisher', 2000),
('Academic Press', 'educational and technical books', 1975);

-- Create BM25 indexes
CREATE INDEX authors_bm25_idx ON authors USING bm25 (id, name, bio) WITH (key_field = 'id');
CREATE INDEX books_bm25_idx ON books USING bm25 (id, title, content) WITH (key_field = 'id');
CREATE INDEX publishers_bm25_idx ON publishers USING bm25 (id, name, description) WITH (key_field = 'id');

-- =============================================================================
-- SUPPORTED CASES: Equi-joins with various join types
-- =============================================================================

SELECT '=== SUPPORTED CASES: Equi-joins with cross-table OR decomposition ===';

-- Test 1: INNER JOIN with equi-join condition
SELECT 'Test 1: INNER JOIN with equi-join condition';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'science' OR b.content @@@ 'technology')
ORDER BY author_score DESC, book_score DESC;

-- Test 2: LEFT JOIN with equi-join condition
SELECT 'Test 2: LEFT JOIN with equi-join condition';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
LEFT JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'mystery' OR b.content @@@ 'romance')
ORDER BY author_score DESC, book_score DESC;

-- Test 3: RIGHT JOIN with equi-join condition
SELECT 'Test 3: RIGHT JOIN with equi-join condition';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
RIGHT JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'fiction' OR b.content @@@ 'algorithms')
ORDER BY author_score DESC, book_score DESC;

-- Test 4: Three-table equi-join (simulated with CROSS JOIN for simplicity)
SELECT 'Test 4: Three-table cross-table OR decomposition';
SELECT 
    a.name as author_name,
    b.title as book_title,
    p.name as publisher_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score,
    paradedb.score(p.id) as publisher_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id
CROSS JOIN publishers p
WHERE (a.bio @@@ 'author' OR b.content @@@ 'science' OR p.description @@@ 'technology')
ORDER BY author_score DESC, book_score DESC, publisher_score DESC;

-- Test 5: Multiple equi-join conditions (AND combined)
SELECT 'Test 5: Multiple equi-join conditions with AND';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id AND a.birth_year < 2000
WHERE (a.bio @@@ 'writer' OR b.content @@@ 'mystery')
ORDER BY author_score DESC, book_score DESC;

-- =============================================================================
-- UNSUPPORTED CASES: Non-equi joins and problematic conditions
-- =============================================================================

SELECT '=== UNSUPPORTED CASES: Non-equi joins and problematic conditions ===';

-- Test 6: CROSS JOIN (no join condition) - should be rejected
SELECT 'Test 6: CROSS JOIN with no join condition';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
CROSS JOIN books b
WHERE (a.bio @@@ 'author' OR b.content @@@ 'mystery')
ORDER BY author_score DESC, book_score DESC;

-- Test 7: INNER JOIN with non-equi condition (<, >, etc.)
SELECT 'Test 7: INNER JOIN with non-equi condition';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.birth_year < b.publication_year
WHERE (a.bio @@@ 'fiction' OR b.content @@@ 'love')
ORDER BY author_score DESC, book_score DESC;

-- Test 8: INNER JOIN with complex non-equi condition
SELECT 'Test 8: INNER JOIN with complex non-equi condition';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.birth_year + 50 > b.publication_year
WHERE (a.bio @@@ 'writer' OR b.content @@@ 'programming')
ORDER BY author_score DESC, book_score DESC;

-- Test 9: INNER JOIN with mixed equi and non-equi conditions
SELECT 'Test 9: INNER JOIN with mixed equi and non-equi conditions';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id AND a.birth_year < b.publication_year
WHERE (a.bio @@@ 'novelist' OR b.content @@@ 'adventure')
ORDER BY author_score DESC, book_score DESC;

-- Test 10: INNER JOIN with BETWEEN condition (range, non-equi)
SELECT 'Test 10: INNER JOIN with BETWEEN condition';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON b.price BETWEEN 20.00 AND 30.00 AND a.id = b.author_id
WHERE (a.bio @@@ 'author' OR b.content @@@ 'romance')
ORDER BY author_score DESC, book_score DESC;

-- =============================================================================
-- EDGE CASES: Complex scenarios
-- =============================================================================

SELECT '=== EDGE CASES: Complex scenarios ===';

-- Test 11: Self-join with equi-join condition
SELECT 'Test 11: Self-join with equi-join condition';
SELECT 
    a1.name as author1_name,
    a2.name as author2_name,
    paradedb.score(a1.id) as author1_score,
    paradedb.score(a2.id) as author2_score
FROM authors a1
INNER JOIN authors a2 ON a1.birth_year = a2.birth_year AND a1.id != a2.id
WHERE (a1.bio @@@ 'fiction' OR a2.bio @@@ 'mystery')
ORDER BY author1_score DESC, author2_score DESC;

-- Test 12: Multiple OR conditions with mixed search and non-search predicates
SELECT 'Test 12: Mixed search and non-search predicates in OR';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'science' OR b.content @@@ 'mystery' OR b.price > 25.00)
ORDER BY author_score DESC, book_score DESC;

-- Test 13: Nested OR with AND conditions
SELECT 'Test 13: Nested OR with AND conditions';
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id
WHERE ((a.bio @@@ 'author' AND a.birth_year > 1965) OR (b.content @@@ 'technology' AND b.publication_year > 2019))
ORDER BY author_score DESC, book_score DESC;

-- Clean up
DROP TABLE IF EXISTS publishers;
DROP TABLE IF EXISTS books;
DROP TABLE IF EXISTS authors; 

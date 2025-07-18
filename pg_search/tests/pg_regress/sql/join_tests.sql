-- JOIN handling test cases
-- This file contains corner cases for JOIN support with @@@ operator and OR conditions
-- Covers: equi vs non-equi joins, cross-table OR, complex boolean logic,
-- edge cases, logical holes, and performance vs correctness scenarios

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- SETUP
-- =============================================================================

DROP TABLE IF EXISTS reviews CASCADE;
DROP TABLE IF EXISTS books CASCADE;
DROP TABLE IF EXISTS authors CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS publishers CASCADE;
DROP TABLE IF EXISTS bridge_table CASCADE;

CREATE TABLE authors (
    id SERIAL PRIMARY KEY,
    name TEXT,
    bio TEXT,
    country TEXT,
    birth_year INTEGER,
    is_active BOOLEAN DEFAULT true
);

CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    is_active BOOLEAN DEFAULT true
);

CREATE TABLE publishers (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    founded_year INTEGER
);

CREATE TABLE books (
    id INT,
    title TEXT,
    content TEXT,
    author_id INTEGER REFERENCES authors(id),
    category_id INTEGER REFERENCES categories(id),
    publisher_id INTEGER REFERENCES publishers(id),
    publication_year INTEGER,
    is_published BOOLEAN DEFAULT true,
    rating DECIMAL(3,2),
    price DECIMAL(10,2),
    PRIMARY KEY (id, author_id)
);

CREATE TABLE reviews (
    id SERIAL PRIMARY KEY,
    book_id INTEGER,
    author_id INTEGER REFERENCES authors(id),  -- reviewer
    content TEXT,
    score INTEGER,
    is_verified BOOLEAN DEFAULT false,
    FOREIGN KEY (book_id, author_id) REFERENCES books(id, author_id)
);

CREATE TABLE bridge_table (
    author_id INTEGER REFERENCES authors(id),
    book_id INTEGER REFERENCES books(id),
    relationship_type TEXT,
    strength INTEGER
);

-- Insert comprehensive test data
INSERT INTO authors (name, bio, country, birth_year, is_active) VALUES
('J.K. Rowling', 'British author famous for Harry Potter magic series', 'UK', 1965, true),
('Stephen King', 'American author of horror and supernatural fiction terror', 'USA', 1947, true),
('Agatha Christie', 'English writer known for detective mystery novels', 'UK', 1890, false),
('George Orwell', 'English novelist and essayist technology writer', 'UK', 1903, false),
('Jane Austen', 'English novelist known for romantic fiction love stories', 'UK', 1775, false),
('John Smith', 'Famous science fiction author smartphone technology', 'USA', 1960, true);

INSERT INTO categories (name, description, is_active) VALUES
('Fantasy', 'Fantasy and magical realism books with magic spells', true),
('Horror', 'Horror and thriller books with terror elements', true),
('Mystery', 'Detective and mystery books with investigation', true),
('Classic', 'Classic literature books with timeless stories', true),
('Romance', 'Romantic fiction books with love stories', false),
('Technology', 'Technology and science publications with innovation', true);

INSERT INTO publishers (name, description, founded_year) VALUES
('TechBooks Publishing', 'technology and science publications with innovation', 1990),
('Mystery House Press', 'crime and mystery novels with detective stories', 1985),
('Romance World', 'romantic fiction publisher with love stories', 2000),
('Academic Press', 'educational and technical books with performance', 1975),
('Magic Books', 'fantasy and magical publications with spells', 1980);

INSERT INTO books (id, title, content, author_id, category_id, publisher_id, publication_year, is_published, rating, price) VALUES
(1, 'Harry Potter Magic', 'A magical story about wizards and magic spells technology', 1, 1, 5, 1997, true, 4.8, 29.99),
(1, 'Harry Potter Horrors', 'A magical story about wizards and magic spells', 2, 1, 5, 1997, true, 4.8, 29.99),
(2, 'The Shining Terror', 'A horror story about supernatural terror events performance', 2, 2, 4, 1977, true, 4.5, 19.99),
(3, 'Murder Mystery Case', 'A detective story with mystery and murder investigation', 3, 3, 2, 1934, true, 4.2, 39.99),
(4, 'Dystopian Future', 'A story about totalitarian surveillance and technology control', 4, 4, 4, 1949, true, 4.7, 24.99),
(5, 'Pride Romance', 'A romantic story about love and prejudice relationships', 5, 5, 3, 1813, false, 4.6, 34.99),
(6, 'Magic Detective', 'A mystery story with magical elements and detective work', 1, 3, 5, 2001, true, 4.1, 21.99),
(7, 'Smartphone Tech', 'Advanced smartphone technology with innovation features', 6, 6, 1, 2020, true, 4.3, 699.99),
(8, 'Future Technologies', 'artificial intelligence machine learning robots performance', 6, 6, 1, 2020, true, 4.4, 89.99);

INSERT INTO reviews (book_id, author_id, content, score, is_verified) VALUES
(1, 2, 'Amazing magical story with great characters and excellent storytelling', 5, true),
(2, 1, 'Terrifying horror story that kept me awake with excellent performance', 4, true),
(3, 4, 'Classic mystery with excellent detective work and investigation', 5, false),
(4, 3, 'Thought-provoking story about surveillance and technology innovation', 4, true),
(5, 2, 'Beautiful romantic story with great character development and love', 5, false),
(6, 5, 'Interesting combination of mystery and magic with storytelling', 4, true),
(7, 1, 'Great smartphone technology review with innovation features', 4, true),
(8, 3, 'Excellent technology book with performance and capabilities', 5, true);

INSERT INTO bridge_table (author_id, book_id, relationship_type, strength) VALUES
(1, 1, 'primary', 10), (2, 2, 'primary', 9), (3, 3, 'primary', 8),
(4, 4, 'primary', 7), (5, 5, 'primary', 6), (6, 7, 'primary', 9),
(1, 6, 'secondary', 5), (6, 8, 'primary', 8);

-- Create BM25 indexes
CREATE INDEX authors_bm25_idx ON authors USING bm25 (id, name, bio, country) WITH (key_field = 'id');
CREATE INDEX books_bm25_idx ON books USING bm25 (id, title, content) WITH (key_field = 'id');
CREATE INDEX reviews_bm25_idx ON reviews USING bm25 (id, content) WITH (key_field = 'id');
CREATE INDEX categories_bm25_idx ON categories USING bm25 (id, name, description) WITH (key_field = 'id');
CREATE INDEX publishers_bm25_idx ON publishers USING bm25 (id, name, description) WITH (key_field = 'id');

-- =============================================================================
-- SECTION 1: Equi-joins with various join types
-- =============================================================================

-- Test 1.1: INNER JOIN with equi-join condition
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'science' OR b.content @@@ 'technology')
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- Test 1.2: LEFT JOIN with equi-join condition
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
LEFT JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'mystery' OR b.content @@@ 'romance')
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- Test 1.3: RIGHT JOIN with equi-join condition
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
RIGHT JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'fiction' OR b.content @@@ 'magic')
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- Test 1.4: Multiple equi-join conditions with AND
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id AND a.birth_year < 2000
WHERE (a.bio @@@ 'writer' OR b.content @@@ 'mystery')
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- =============================================================================
-- SECTION 2: Non-equi joins and problematic conditions
-- =============================================================================

-- Test 2.1: CROSS JOIN (no join condition) - should be rejected
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
CROSS JOIN books b
WHERE (a.bio @@@ 'author' OR b.content @@@ 'mystery')
ORDER BY a.id, b.id, author_score DESC, book_score DESC
LIMIT 10;

-- Test 2.2: INNER JOIN with non-equi condition (<, >, etc.)
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.birth_year < b.publication_year
WHERE (a.bio @@@ 'fiction' OR b.content @@@ 'love')
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- Test 2.3: INNER JOIN with complex non-equi condition
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.birth_year + 50 > b.publication_year
WHERE (a.bio @@@ 'writer' OR b.content @@@ 'programming')
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- Test 2.4: INNER JOIN with BETWEEN condition (range, non-equi)
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON b.price BETWEEN 20.00 AND 30.00 AND a.id = b.author_id
WHERE (a.bio @@@ 'author' OR b.content @@@ 'romance')
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- =============================================================================
-- SECTION 3: CROSS-TABLE OR TESTS
-- =============================================================================

-- Test 3.1: Basic cross-table OR
SELECT 
    a.name as author_name,
    b.content as book_content,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
CROSS JOIN books b
WHERE (a.bio @@@ 'smartphone' OR b.content @@@ 'performance')
ORDER BY a.id, b.id, author_score DESC, book_score DESC
LIMIT 10;

-- Test 3.2: Three-table OR
SELECT 
    a.name as author_name,
    b.title as book_title,
    c.name as category_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score,
    paradedb.score(c.id) as category_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id
CROSS JOIN categories c
WHERE (a.bio @@@ 'author' OR b.content @@@ 'science' OR c.description @@@ 'technology')
ORDER BY a.id, b.id, c.id, author_score DESC, book_score DESC, category_score DESC
LIMIT 10;

-- Test 3.3: Multiple conditions per relation in OR
SELECT 
    a.name as author_name,
    a.country as author_country,
    b.content as book_content,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
CROSS JOIN books b
WHERE (a.bio @@@ 'smartphone' OR a.country @@@ 'British' OR b.content @@@ 'performance')
ORDER BY a.id, b.id, author_score DESC, book_score DESC
LIMIT 10;

-- =============================================================================
-- SECTION 4: COMPLEX BOOLEAN LOGIC TESTS
-- =============================================================================

-- Test 4.1: Mixed search and non-search predicates in OR
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'science' OR b.content @@@ 'mystery' OR b.price > 25.00)
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- Test 4.2: Nested AND/OR combinations
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
INNER JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'smartphone' AND a.birth_year > 1950) 
   OR (b.content @@@ 'magic' AND b.publication_year > 1980)
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- Test 4.3: Complex boolean logic across three tables
SELECT 
    a.name as author_name,
    b.title as book_title,
    r.content as review_content,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score,
    paradedb.score(r.id) as review_score
FROM authors a
JOIN books b ON a.id = b.author_id
JOIN reviews r ON b.id = r.book_id
WHERE (a.bio @@@ 'British' AND b.is_published = true) 
   OR (b.content @@@ 'horror' AND r.score >= 4)
ORDER BY a.id, b.id, r.id, author_score DESC, book_score DESC, review_score DESC;

-- Test 4.4: Intelligent partial salvage of AND expressions
SELECT 
    a.name as author_name,
    paradedb.score(a.id) as author_score
FROM authors a
JOIN categories c ON a.id = c.id
WHERE (a.bio @@@ 'laptop')
  AND (a.birth_year > 1000)
  AND (c.name @@@ 'Electronics')
ORDER BY a.id, author_score DESC;

-- =============================================================================
-- SECTION 5: EDGE CASES AND LOGICAL HOLES
-- =============================================================================

-- Test 5.1: Self-join with equi-join condition
SELECT 
    a1.name as author1_name,
    a2.name as author2_name,
    paradedb.score(a1.id) as author1_score,
    paradedb.score(a2.id) as author2_score
FROM authors a1
INNER JOIN authors a2 ON a1.birth_year = a2.birth_year AND a1.id != a2.id
WHERE (a1.bio @@@ 'fiction' OR a2.bio @@@ 'mystery')
ORDER BY a1.id, a2.id, author1_score DESC, author2_score DESC;

-- Test 5.2: Variable scope violation test
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
JOIN books b ON a.id = b.author_id
WHERE a.bio @@@ 'author' AND b.category_id = 1
ORDER BY a.id, b.id, author_score DESC, book_score DESC;

-- Test 5.3: LEFT JOIN semantics test
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
LEFT JOIN books b ON a.id = b.author_id
WHERE a.bio @@@ 'author' OR b.content @@@ 'story'
ORDER BY a.id, b.id;

-- Test 5.4: NULL-generating join test
SELECT 
    a.name as author_name,
    b.title as book_title,
    c.name as category_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score,
    paradedb.score(c.id) as category_score
FROM authors a
LEFT JOIN books b ON a.id = b.author_id
LEFT JOIN categories c ON b.category_id = c.id
WHERE a.bio @@@ 'author' 
   OR (b.content @@@ 'story' AND c.name @@@ 'Fantasy')
ORDER BY a.id, b.id, c.id
LIMIT 15;

-- Test 5.5: Complex join forcing joininfo conditions
SELECT 
    a.name as author_name,
    b.title as book_title,
    br.relationship_type,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
JOIN bridge_table br ON a.id = br.author_id
JOIN books b ON b.id = br.book_id
WHERE (a.bio @@@ 'smartphone' AND b.content @@@ 'advanced')
   OR (a.birth_year > 1900 AND b.rating > 4.0)
ORDER BY a.id, b.id, author_score DESC;

-- =============================================================================
-- SECTION 6: PERFORMANCE VS CORRECTNESS TESTS
-- =============================================================================

-- Test 6.1: Score consistency check - direct vs join query
SELECT 
    a.name as author_name,
    paradedb.score(a.id) as author_score
FROM authors a
WHERE a.bio @@@ 'author'
ORDER BY a.id, author_score DESC;

SELECT 
    a.name as author_name,
    paradedb.score(a.id) as author_score
FROM authors a
JOIN books b ON a.id = b.author_id
WHERE a.bio @@@ 'author'
ORDER BY a.id, author_score DESC;

-- Test 6.2: Performance vs correctness trade-off
SELECT 
    COUNT(*) as total_results,
    AVG(paradedb.score(a.id)) as avg_author_score,
    AVG(paradedb.score(b.id)) as avg_book_score
FROM authors a
JOIN books b ON a.id = b.author_id
WHERE (a.bio @@@ 'author' OR b.content @@@ 'story')
  AND (a.is_active = true OR b.is_published = true);

-- Test 6.3: Unsafe conditions that cannot be pushed down
SELECT 
    a.name as author_name,
    b.title as book_title,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score
FROM authors a
CROSS JOIN books b
WHERE (a.bio @@@ 'smartphone' OR a.birth_year = b.publication_year)
ORDER BY a.id, b.id, author_score DESC, book_score DESC
LIMIT 5;

-- =============================================================================
-- SECTION 7: Misc.
-- =============================================================================

-- Test 7.1: Multiple diagnostic types in one query
SELECT 
    a.name as author_name,
    b.title as book_title,
    r.content as review_content,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score,
    paradedb.score(r.id) as review_score
FROM authors a
JOIN books b ON a.id = b.author_id
LEFT JOIN reviews r ON b.id = r.book_id
WHERE (
    (a.bio @@@ 'laptop' AND a.birth_year > 1000)
    OR 
    (b.content @@@ 'Electronics' AND r.score > 4)
    OR
    (a.is_active = true AND b.is_published = true)
)
ORDER BY a.id, b.id, r.id, author_score DESC, book_score DESC
LIMIT 10;

-- Test 7.2: Conservative OR handling demonstration
SELECT 
    a.name as author_name,
    c.name as category_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(c.id) as category_score
FROM authors a
JOIN books b ON a.id = b.author_id
JOIN categories c ON b.category_id = c.id
WHERE (a.bio @@@ 'smartphone')    -- Safe condition on authors
   OR (c.description @@@ 'electronic')    -- External condition on categories
ORDER BY a.id, c.id, author_score DESC, category_score DESC;

-- =============================================================================
-- SECTION 8: VERIFICATION AND COMPARISON TESTS
-- =============================================================================

-- Test 8.1: Single-table queries for comparison
SELECT 'Single table A - smartphone' as query_type, 
       a.name, paradedb.score(a.id) as score
FROM authors a 
WHERE a.bio @@@ 'smartphone'
UNION ALL
SELECT 'Single table B - performance' as query_type,
       b.title, paradedb.score(b.id) as score  
FROM books b
WHERE b.content @@@ 'performance'
ORDER BY score DESC;

-- Test 8.2: Complex real-world scenario
SELECT 
    a.name as author_name,
    b.title as book_title,
    c.name as category_name,
    p.name as publisher_name,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score,
    paradedb.score(c.id) as category_score,
    paradedb.score(p.id) as publisher_score
FROM authors a
LEFT JOIN books b ON a.id = b.author_id
LEFT JOIN categories c ON b.category_id = c.id
LEFT JOIN publishers p ON b.publisher_id = p.id
WHERE (a.bio @@@ 'technology' OR a.country @@@ 'British')
   OR (b.content @@@ 'performance' OR b.title @@@ 'magic')
   OR (c.description @@@ 'innovation' OR c.name @@@ 'Fantasy')
   OR (p.description @@@ 'technology' OR p.name @@@ 'Academic')
ORDER BY a.id, b.id, c.id, p.id, author_score DESC, book_score DESC
LIMIT 15;

-- Cleanup
DROP TABLE IF EXISTS bridge_table;
DROP TABLE IF EXISTS reviews;
DROP TABLE IF EXISTS books;
DROP TABLE IF EXISTS publishers;
DROP TABLE IF EXISTS categories;
DROP TABLE IF EXISTS authors;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan; 

-- Phase 1 TopN Join Optimization Test
CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_custom_join = true;
SET paradedb.enable_topn_join_optimization = true;

-- Create test tables with more data to see the optimization benefits
CREATE TABLE books_phase1 (
    id INTEGER PRIMARY KEY,
    title TEXT,
    author_id INTEGER,
    content TEXT
);

CREATE TABLE authors_phase1 (
    id INTEGER PRIMARY KEY,
    name TEXT,
    bio TEXT
);

-- Insert test data
INSERT INTO books_phase1 VALUES 
(1, 'The Hitchhiker''s Guide to the Galaxy', 1, 'science fiction comedy book'),
(2, 'Foundation', 2, 'science fiction empire galactic history'),
(3, 'Dune', 3, 'science fiction desert planet spice politics'),
(4, 'Neuromancer', 4, 'cyberpunk science fiction matrix virtual reality'),
(5, 'The Left Hand of Darkness', 5, 'science fiction gender society alien culture'),
(6, 'Hyperion', 6, 'science fiction poetry pilgrimage future technology'),
(7, 'Ender''s Game', 7, 'science fiction children war strategy aliens'),
(8, 'I, Robot', 2, 'science fiction robots artificial intelligence ethics');

INSERT INTO authors_phase1 VALUES 
(1, 'Douglas Adams', 'British author known for comedy science fiction'),
(2, 'Isaac Asimov', 'Prolific author of science fiction and popular science'),
(3, 'Frank Herbert', 'American science fiction author best known for Dune'),
(4, 'William Gibson', 'American-Canadian author pioneering cyberpunk genre'),
(5, 'Ursula K. Le Guin', 'American author of science fiction and fantasy'),
(6, 'Dan Simmons', 'American science fiction and horror author'),
(7, 'Orson Scott Card', 'American author of science fiction and fantasy'),
(8, 'Philip K. Dick', 'American science fiction author exploring reality and consciousness');

-- Create BM25 indexes
CREATE INDEX books_phase1_idx ON books_phase1
USING bm25 (id, title, author_id, content)
WITH (
    key_field = 'id',
    text_fields = '{"content": {"tokenizer": {"type": "default"}}, "title": {"tokenizer": {"type": "default"}}}',
    numeric_fields = '{"author_id": {}}'
);

CREATE INDEX authors_phase1_idx ON authors_phase1
USING bm25 (id, name, bio)
WITH (
    key_field = 'id',
    text_fields = '{"bio": {"tokenizer": {"type": "default"}}, "name": {"tokenizer": {"type": "default"}}}'
);

-- Test 1: Small TopN join that should show Phase 1 optimization
-- Look for the "Phase 1 OPTIMIZATION" log message showing bounded priority queue
SELECT 'Test 1: TopN Join with LIMIT 3 - Should show Phase 1 optimization';
SELECT b.title, a.name 
FROM books_phase1 b 
JOIN authors_phase1 a ON b.author_id = a.id 
WHERE b.content @@@ 'science fiction' 
  AND a.bio @@@ 'author' 
ORDER BY b.id DESC 
LIMIT 3;

-- Test 2: Larger dataset to really see the benefit
SELECT 'Test 2: TopN Join with LIMIT 2 - Should process minimal combinations';
SELECT b.title, a.name
FROM books_phase1 b 
JOIN authors_phase1 a ON b.author_id = a.id 
WHERE b.content @@@ 'science' 
  AND a.bio @@@ 'science fiction'
ORDER BY b.id DESC 
LIMIT 2;

-- Test 3: Very small limit to test early termination
SELECT 'Test 3: TopN Join with LIMIT 1 - Should terminate very early';
SELECT b.title
FROM books_phase1 b 
JOIN authors_phase1 a ON b.author_id = a.id 
WHERE b.content @@@ 'fiction' 
  AND a.bio @@@ 'author'
ORDER BY b.id DESC 
LIMIT 1;

-- Cleanup
DROP TABLE books_phase1 CASCADE;
DROP TABLE authors_phase1 CASCADE; 

-- TopN Join Test
-- This file demonstrates the TopN join optimization that reduces join processing
-- from potentially billions of combinations to only the top N needed for LIMIT clauses.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_custom_join = true;
SET paradedb.enable_topn_join_optimization = true;

-- Setup test tables with BM25 indexes
CREATE TABLE books_topn (
    id INTEGER PRIMARY KEY,
    title TEXT,
    author_id INTEGER,
    content TEXT
);

CREATE TABLE authors_topn (
    id INTEGER PRIMARY KEY,
    name TEXT,
    bio TEXT
);


CREATE INDEX books_topn_idx ON books_topn
USING bm25 ("id",
    "title",
    "author_id",
    "content"
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
        "author_id": {}
    }'
);

CREATE INDEX authors_topn_idx ON authors_topn
USING bm25 ("id",
    "name",
    "bio"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "name": {
            "tokenizer": {"type": "default"}
        },
        "bio": {
            "tokenizer": {"type": "default"}
        }
    }'
);

-- Insert test data
INSERT INTO authors_topn (id, name, bio) VALUES
(1, 'J.K. Rowling', 'British author famous for Harry Potter series magic fantasy'),
(2, 'Stephen King', 'American author known for horror supernatural fiction writing'),
(3, 'Agatha Christie', 'British detective mystery crime fiction writer novelist'),
(4, 'Isaac Asimov', 'Science fiction robotics foundation series author scientist'),
(5, 'George Orwell', 'British author dystopian political fiction nineteen eighty-four');

INSERT INTO books_topn (id, title, author_id, content) VALUES
(1, 'Harry Potter and the Philosophers Stone', 1, 'Young wizard magic school adventure fantasy story'),
(2, 'The Shining', 2, 'Horror psychological supernatural haunted hotel winter isolation'),
(3, 'Murder on the Orient Express', 3, 'Detective mystery crime investigation train passengers'),
(4, 'Foundation', 4, 'Science fiction galactic empire psychohistory mathematics prediction'),
(5, '1984', 5, 'Dystopian surveillance totalitarian government thought control society'),
(6, 'It', 2, 'Horror supernatural entity children small town terror fear'),
(7, 'And Then There Were None', 3, 'Mystery crime isolated island ten guests murder'),
(8, 'I Robot', 4, 'Science fiction artificial intelligence robotics three laws'),
(9, 'Animal Farm', 5, 'Political allegory farm animals revolution dictatorship power'),
(10, 'The Stand', 2, 'Post-apocalyptic pandemic survival good versus evil battle');

-- Test 1: TopN Join with LIMIT 5 
-- This should trigger TopN optimization, processing only ~15 combinations instead of 50
EXPLAIN (ANALYZE, BUFFERS) 
SELECT b.title, a.name, b.content
FROM books_topn b
JOIN authors_topn a ON b.author_id = a.id
WHERE b.content @@@ 'science fiction'
  AND a.bio @@@ 'author'
ORDER BY b.content @@@ 'science fiction' DESC
LIMIT 5;

-- Test 2: Standard join without LIMIT for comparison
-- This should use standard join processing
EXPLAIN (ANALYZE, BUFFERS)
SELECT b.title, a.name, b.content  
FROM books_topn b
JOIN authors_topn a ON b.author_id = a.id
WHERE b.content @@@ 'fiction'
  AND a.bio @@@ 'author'
ORDER BY b.content @@@ 'fiction' DESC;

-- Test 3: TopN Join with composite relation (nested join)
-- This tests TopN optimization with one side being a composite relation
EXPLAIN (ANALYZE, BUFFERS)
SELECT outer_result.title, outer_result.author_name, outer_result.content
FROM (
    SELECT b.title, a.name as author_name, b.content, b.id
    FROM books_topn b  
    JOIN authors_topn a ON b.author_id = a.id
    WHERE a.bio @@@ 'British'
) outer_result
JOIN books_topn b2 ON outer_result.id != b2.id
WHERE b2.content @@@ 'horror'
ORDER BY b2.content @@@ 'horror' DESC
LIMIT 3;

-- Test 4: Verify TopN results are correct
-- Compare TopN limited results with full results
WITH full_results AS (
    SELECT b.title, a.name, 
           b.content @@@ 'magic fantasy' as distance
    FROM books_topn b
    JOIN authors_topn a ON b.author_id = a.id  
    WHERE b.content @@@ 'magic fantasy'
      AND a.bio @@@ 'author'
    ORDER BY distance DESC
),
topn_results AS (
    SELECT b.title, a.name,
           b.content @@@ 'magic fantasy' as distance
    FROM books_topn b
    JOIN authors_topn a ON b.author_id = a.id
    WHERE b.content @@@ 'magic fantasy' 
      AND a.bio @@@ 'author'
    ORDER BY distance DESC
    LIMIT 2
)
SELECT 'Full Results' as result_type, * FROM full_results
UNION ALL
SELECT 'TopN Results' as result_type, * FROM topn_results
ORDER BY result_type, distance DESC;

-- Cleanup
DROP TABLE IF EXISTS books_topn CASCADE;
DROP TABLE IF EXISTS authors_topn CASCADE; 

RESET paradedb.enable_custom_join;
RESET paradedb.enable_topn_join_optimization;

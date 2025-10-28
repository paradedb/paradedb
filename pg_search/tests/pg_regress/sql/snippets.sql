-- tests for pdb.snippets

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS snippets_test;
CREATE TABLE snippets_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

-- Insert data with multiple potential snippets
INSERT INTO snippets_test (id, content) VALUES
(1, 'The quick brown fox jumps over the lazy dog. The dog is very lazy. The fox is quick.'),
(2, 'A lazy dog is a happy dog. Dogs are the best, especially a lazy one.'),
(3, 'Foxes and dogs are not friends. A quick fox is a clever fox.'),
(4, 'This text does not contain the search words.'),
(5, 'The lazy brown dog, and the quick red fox. The dog and fox are here.'),
(6, 'A sentence with no matching words.'),
(7, 'A test sentence for testing. Another test sentence.');

CREATE INDEX idx_snippets_test ON snippets_test USING bm25 (id, content) WITH (key_field = 'id');

-- =====================================================================
-- Basic tests for pdb.snippets
-- =====================================================================

\echo '--- Basic pdb.snippets tests ---'

-- Basic usage with a single keyword, multiple occurrences
SELECT id, pdb.snippets(content) FROM snippets_test WHERE content @@@ 'lazy' ORDER BY id;

-- Multiple keywords (OR)
SELECT id, pdb.snippets(content) FROM snippets_test WHERE content @@@ 'fox OR dog' ORDER BY id;

-- Phrase search
SELECT id, pdb.snippets(content) FROM snippets_test WHERE content @@@ '"lazy dog"' ORDER BY id;

-- =====================================================================
-- Tests for pdb.snippets with arguments
-- =====================================================================

\echo '--- pdb.snippets with arguments ---'

-- Custom tags
SELECT id, pdb.snippets(content, start_tag => '<em>', end_tag => '</em>') FROM snippets_test WHERE content @@@ 'quick' ORDER BY id;

-- =====================================================================
-- Tests for pdb.snippets with limit and offset
-- =====================================================================

\echo '--- pdb.snippets with limit and offset ---'

-- With a small max_num_chars, we can generate multiple snippets per document.
-- This query should produce 2 snippets for id=1, 2 for id=3, and 2 for id=5
SELECT id, pdb.snippets(content, max_num_chars => 25) FROM snippets_test WHERE content @@@ 'fox' ORDER BY id;

-- Test limit: should return only the first snippet for each document
SELECT id, pdb.snippets(content, max_num_chars => 25, "limit" => 1) FROM snippets_test WHERE content @@@ 'fox' ORDER BY id;

-- Test offset: should return the second snippet for each document
SELECT id, pdb.snippets(content, max_num_chars => 25, "limit" => 1, "offset" => 1) FROM snippets_test WHERE content @@@ 'fox' ORDER BY id;

-- Test offset without limit: should return all snippets starting from the second one
SELECT id, pdb.snippets(content, max_num_chars => 25, "offset" => 1) FROM snippets_test WHERE content @@@ 'fox' ORDER BY id;

-- Test offset beyond the number of snippets: should return empty array
SELECT id, pdb.snippets(content, max_num_chars => 25, "offset" => 2) FROM snippets_test WHERE content @@@ 'fox' ORDER BY id;

-- Test with a different max_num_chars to ensure limit and offset are behaving correctly
-- This should produce 2 snippets for id=1
SELECT id, pdb.snippets(content, max_num_chars => 40) FROM snippets_test WHERE content @@@ 'dog' ORDER BY id;
SELECT id, pdb.snippets(content, max_num_chars => 40, "limit" => 1) FROM snippets_test WHERE content @@@ 'dog' ORDER BY id;
SELECT id, pdb.snippets(content, max_num_chars => 40, "limit" => 1, "offset" => 1) FROM snippets_test WHERE content @@@ 'dog' ORDER BY id;
SELECT id, pdb.snippets(content, max_num_chars => 40, "offset" => 1) FROM snippets_test WHERE content @@@ 'dog' ORDER BY id;
SELECT id, pdb.snippets(content, max_num_chars => 40, "offset" => 2) FROM snippets_test WHERE content @@@ 'dog' ORDER BY id;

-- Test `limit` and `offset` on a query that returns a single snippet by default
SELECT id, pdb.snippets(content, "limit" => 1) FROM snippets_test WHERE content @@@ 'test' ORDER BY id;
SELECT id, pdb.snippets(content, "limit" => 1, "offset" => 1) FROM snippets_test WHERE content @@@ 'test' ORDER BY id;

-- Test with multiple search terms, small max_num_chars, and limit/offset
-- This should generate a lot of snippets
SELECT id, pdb.snippets(content, max_num_chars => 20) FROM snippets_test WHERE content @@@ 'fox OR dog OR lazy OR quick' ORDER BY id;

-- With limit
SELECT id, pdb.snippets(content, max_num_chars => 20, "limit" => 2) FROM snippets_test WHERE content @@@ 'fox OR dog OR lazy OR quick' ORDER BY id;

-- With limit and offset
SELECT id, pdb.snippets(content, max_num_chars => 20, "limit" => 2, "offset" => 1) FROM snippets_test WHERE content @@@ 'fox OR dog OR lazy OR quick' ORDER BY id;

-- With offset
SELECT id, pdb.snippets(content, max_num_chars => 20, "offset" => 3) FROM snippets_test WHERE content @@@ 'fox OR dog OR lazy OR quick' ORDER BY id;

-- =====================================================================
-- Tests for pdb.snippets with sort_by
-- =====================================================================

\echo '--- pdb.snippets with sort_by ---'

INSERT INTO snippets_test (id, content) VALUES (8, 'term1 term2. some other text. term1 term1 term2.');

-- Test with sort_by => 'score' (default)
-- The second snippet has more matches, so it should be first
SELECT id, pdb.snippets(content, max_num_chars => 20, sort_by => 'score') FROM snippets_test WHERE content @@@ 'term1 OR term2' AND id = 8;

-- Test with sort_by => 'position'
-- Snippets should be in order of appearance
SELECT id, pdb.snippets(content, max_num_chars => 20, sort_by => 'position') FROM snippets_test WHERE content @@@ 'term1 OR term2' AND id = 8;

-- Test with an invalid sort_by value
SELECT id, pdb.snippets(content, sort_by => 'invalid') FROM snippets_test WHERE content @@@ 'lazy' AND id = 1;

-- Cleanup
DROP TABLE snippets_test;

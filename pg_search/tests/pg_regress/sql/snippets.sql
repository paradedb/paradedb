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

-- =====================================================================
-- Parameterized snippet arguments (issue: snippet panics on 6th EXECUTE)
--
-- Pre-fix: every formatting argument (start_tag, end_tag, max_num_chars,
-- limit, offset, sort_by) had to be a Const. In GENERIC plan mode (the
-- 6th+ EXECUTE of a prepared statement), Params survive into the planner
-- and the snippet extractor panicked with "arguments must be literals".
--
-- We exercise both CUSTOM and GENERIC modes against the same table so
-- the expected output proves they produce identical results.
-- =====================================================================

\echo '--- Parameterized snippet arguments (CUSTOM vs GENERIC) ---'

-- pdb.snippet with parameterized start_tag and end_tag
SET plan_cache_mode = force_custom_plan;
PREPARE snip_param_c(text, text, text) AS
SELECT id, pdb.snippet(content, $2, $3)
FROM snippets_test
WHERE content @@@ $1
ORDER BY id;
EXECUTE snip_param_c('lazy', '[', ']');
DEALLOCATE snip_param_c;

SET plan_cache_mode = force_generic_plan;
PREPARE snip_param_g(text, text, text) AS
SELECT id, pdb.snippet(content, $2, $3)
FROM snippets_test
WHERE content @@@ $1
ORDER BY id;
EXECUTE snip_param_g('lazy', '[', ']');
DEALLOCATE snip_param_g;

-- pdb.snippets with parameterized sort_by + max_num_chars
SET plan_cache_mode = force_custom_plan;
PREPARE snips_param_c(text, int, text) AS
SELECT id, pdb.snippets(content, max_num_chars => $2, sort_by => $3)
FROM snippets_test
WHERE content @@@ $1 AND id = 8
ORDER BY id;
EXECUTE snips_param_c('term1 OR term2', 20, 'position');
DEALLOCATE snips_param_c;

SET plan_cache_mode = force_generic_plan;
PREPARE snips_param_g(text, int, text) AS
SELECT id, pdb.snippets(content, max_num_chars => $2, sort_by => $3)
FROM snippets_test
WHERE content @@@ $1 AND id = 8
ORDER BY id;
EXECUTE snips_param_g('term1 OR term2', 20, 'position');
DEALLOCATE snips_param_g;

-- pdb.snippet_positions with parameterized limit and offset.
-- The point of this test is "doesn't panic on 6th EXECUTE in GENERIC mode".
SET plan_cache_mode = force_custom_plan;
PREPARE snip_pos_c(text, int, int) AS
SELECT id, pdb.snippet_positions(content, $2, $3) IS NOT NULL AS has_positions
FROM snippets_test
WHERE content @@@ $1 AND id = 1;
EXECUTE snip_pos_c('lazy', 5, 0);
DEALLOCATE snip_pos_c;

SET plan_cache_mode = force_generic_plan;
PREPARE snip_pos_g(text, int, int) AS
SELECT id, pdb.snippet_positions(content, $2, $3) IS NOT NULL AS has_positions
FROM snippets_test
WHERE content @@@ $1 AND id = 1;
EXECUTE snip_pos_g('lazy', 5, 0);
DEALLOCATE snip_pos_g;

-- Parameterized sort_by with an invalid value must still error at exec time
-- (validation moved from planning to execution to support Param values).
SET plan_cache_mode = force_generic_plan;
PREPARE snips_bad_sort_g(text, text) AS
SELECT id, pdb.snippets(content, sort_by => $2)
FROM snippets_test
WHERE content @@@ $1 AND id = 1;
\set VERBOSITY terse
SELECT 'expecting error from pdb.snippets sort_by validation:';
EXECUTE snips_bad_sort_g('lazy', 'invalid');
\set VERBOSITY default
DEALLOCATE snips_bad_sort_g;

RESET plan_cache_mode;

-- Cleanup
DROP TABLE snippets_test;

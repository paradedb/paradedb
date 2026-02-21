\i common/common_setup.sql

-- Test search_tokenizer: use a different tokenizer at search time vs index time.
--
-- The iconic use case is autocomplete:
--   Index time (edge ngram):  "shoes" → s, sh, sho, shoe, shoes
--   Search time (unicode_words): "sho"  → sho
--
-- Without search_tokenizer, typing "sho" would also be edge-ngrammed into
-- s, sh, sho — matching way too many documents.

DROP TABLE IF EXISTS autocomplete;
CREATE TABLE autocomplete (
    id serial8 NOT NULL PRIMARY KEY,
    title text
);
INSERT INTO autocomplete (title) VALUES
    ('shoes'),
    ('shirt'),
    ('shorts'),
    ('shoelaces'),
    ('socks');

-- Edge ngram (prefix_only=true) at index time, unicode_words at search time
CREATE INDEX idx_autocomplete ON autocomplete USING bm25
    (id, (title::pdb.ngram(1, 10, 'prefix_only=true')))
    WITH (key_field = 'id', search_tokenizer = 'unicode_words');

-- "sho" stays as one token at search time → matches only titles whose prefix ngrams include "sho"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT id, title FROM autocomplete WHERE title ||| 'sho' ORDER BY id;
SELECT id, title FROM autocomplete WHERE title ||| 'sho' ORDER BY id;

-- "s" stays as one token → matches every title starting with s
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT id, title FROM autocomplete WHERE title ||| 's' ORDER BY id;
SELECT id, title FROM autocomplete WHERE title ||| 's' ORDER BY id;

-- Explicit tokenizer cast on the query overrides search_tokenizer
-- "sho" gets edge-ngrammed into s, sh, sho → matches all 5 titles (same as plain index)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT id, title FROM autocomplete WHERE title ||| 'sho'::pdb.ngram(1, 10, 'prefix_only=true') ORDER BY id;
SELECT id, title FROM autocomplete WHERE title ||| 'sho'::pdb.ngram(1, 10, 'prefix_only=true') ORDER BY id;

-- Now create the SAME index WITHOUT search_tokenizer to see the difference
DROP TABLE IF EXISTS autocomplete_plain;
CREATE TABLE autocomplete_plain (
    id serial8 NOT NULL PRIMARY KEY,
    title text
);
INSERT INTO autocomplete_plain (title) VALUES
    ('shoes'),
    ('shirt'),
    ('shorts'),
    ('shoelaces'),
    ('socks');

CREATE INDEX idx_autocomplete_plain ON autocomplete_plain USING bm25
    (id, (title::pdb.ngram(1, 10, 'prefix_only=true')))
    WITH (key_field = 'id');

-- Without search_tokenizer, "sho" gets edge-ngrammed into s, sh, sho at query time
-- "s" alone matches ALL titles — way too broad
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT id, title FROM autocomplete_plain WHERE title ||| 'sho' ORDER BY id;
SELECT id, title FROM autocomplete_plain WHERE title ||| 'sho' ORDER BY id;

-- ============================================================
-- Test parameterized search_tokenizer expressions
-- ============================================================

-- Test 1: search_tokenizer with filter overrides (lowercase=false)
-- Index with simple (default lowercase=true), search with simple(lowercase=false)
-- "Running" won't match index tokens (all lowered) because search doesn't lower it
-- "running" will match because it's already lowercase
DROP TABLE IF EXISTS case_test;
CREATE TABLE case_test (
    id serial8 NOT NULL PRIMARY KEY,
    description text
);
INSERT INTO case_test (description) VALUES
    ('Running Shoes'),
    ('running fast'),
    ('RUNNING LATE');

CREATE INDEX idx_case_test ON case_test USING bm25
    (id, description)
    WITH (key_field = 'id', search_tokenizer = 'simple(lowercase=false)');

-- "Running" not lowercased at search time → doesn't match lowered index tokens → 0 rows
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT id, description FROM case_test WHERE description ||| 'Running' ORDER BY id;
SELECT id, description FROM case_test WHERE description ||| 'Running' ORDER BY id;

-- "running" is already lowercase → matches all 3 rows
SELECT id, description FROM case_test WHERE description ||| 'running' ORDER BY id;

-- Test 2: search_tokenizer with combined params on ngram index
-- ngram index + unicode_words(lowercase=false) search tokenizer
-- Verifies both the tokenizer switch and filter params work together
DROP TABLE IF EXISTS combined_test;
CREATE TABLE combined_test (
    id serial8 NOT NULL PRIMARY KEY,
    content text
);
INSERT INTO combined_test (content) VALUES
    ('Hello World'),
    ('hello there'),
    ('HELLO AGAIN');

CREATE INDEX idx_combined ON combined_test USING bm25
    (id, (content::pdb.ngram(1, 10, 'prefix_only=true')))
    WITH (key_field = 'id', search_tokenizer = 'unicode_words(lowercase=false)');

-- "Hello" not lowered at search time → no match against lowered ngram prefixes
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) SELECT id, content FROM combined_test WHERE content ||| 'Hello' ORDER BY id;
SELECT id, content FROM combined_test WHERE content ||| 'Hello' ORDER BY id;

-- "hello" already lowercase → matches prefixes in the ngram index
SELECT id, content FROM combined_test WHERE content ||| 'hello' ORDER BY id;

-- Cleanup
DROP TABLE autocomplete;
DROP TABLE autocomplete_plain;
DROP TABLE case_test;
DROP TABLE combined_test;

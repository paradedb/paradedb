\i common/common_setup.sql

-- Test index-level search_tokenizer as a WITH option.
--
-- The search_tokenizer WITH option sets a default search-time tokenizer
-- for all text/JSON fields in the index.

-------------------------------------------------------------
-- Test 1: Basic index-level search_tokenizer
-------------------------------------------------------------

DROP TABLE IF EXISTS autocomplete;
CREATE TABLE autocomplete (
    id serial8 NOT NULL PRIMARY KEY,
    title text
);
INSERT INTO autocomplete (title) VALUES
    ('shoes'), ('shirt'), ('shorts'), ('shoelaces'), ('socks');

CREATE INDEX idx_autocomplete ON autocomplete USING bm25
    (id, (title::pdb.ngram(1, 10, 'prefix_only=true')))
    WITH (key_field = 'id', search_tokenizer = 'unicode_words');

-- "sho" stays as one token at search time -> matches titles with prefix "sho"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, title FROM autocomplete WHERE title ||| 'sho' ORDER BY id;
SELECT id, title FROM autocomplete WHERE title ||| 'sho' ORDER BY id;

-- "s" stays as one token -> matches every title starting with s
SELECT id, title FROM autocomplete WHERE title ||| 's' ORDER BY id;

-------------------------------------------------------------
-- Test 2: Query-level tokenizer cast overrides index-level
-------------------------------------------------------------

-- On autocomplete table: force edge ngram tokenization at query time
-- "sho" gets ngrammed into s, sh, sho -> matches all 5 titles
SELECT id, title FROM autocomplete WHERE title ||| 'sho'::pdb.ngram(1, 10, 'prefix_only=true') ORDER BY id;

-------------------------------------------------------------
-- Test 3: Parameterized expression as WITH option
-------------------------------------------------------------

DROP TABLE IF EXISTS param_test;
CREATE TABLE param_test (
    id serial8 NOT NULL PRIMARY KEY,
    content text
);
INSERT INTO param_test (content) VALUES
    ('Running Fast'), ('running slow'), ('RUNNING late');

CREATE INDEX idx_param ON param_test USING bm25
    (id, content)
    WITH (key_field = 'id', search_tokenizer = 'simple(lowercase=false)');

-- "Running" not lowered at search time -> no match against lowered index tokens -> 0 rows
SELECT id, content FROM param_test WHERE content ||| 'Running' ORDER BY id;

-- "running" already lowercase -> matches all 3 rows
SELECT id, content FROM param_test WHERE content ||| 'running' ORDER BY id;

-------------------------------------------------------------
-- Test 4: search_tokenizer rejected as typmod param
-------------------------------------------------------------

-- search_tokenizer should only be set as a WITH option, not per-field
CREATE INDEX idx_bad ON autocomplete
    USING bm25 (id, (title::pdb.ngram(1, 10, 'search_tokenizer=unicode_words')))
    WITH (key_field = 'id');

-------------------------------------------------------------
-- Cleanup
-------------------------------------------------------------

DROP TABLE autocomplete;
DROP TABLE param_test;

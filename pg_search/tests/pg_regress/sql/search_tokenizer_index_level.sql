\i common/common_setup.sql

-- Test index-level search_tokenizer as a WITH option.
--
-- The search_tokenizer WITH option sets a default search-time tokenizer
-- for all text/JSON fields. Per-field search_tokenizer in typmod overrides it.

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
-- Test 2: Per-field override takes precedence
-------------------------------------------------------------

DROP TABLE IF EXISTS override_test;
CREATE TABLE override_test (
    id serial8 NOT NULL PRIMARY KEY,
    title text,
    subtitle text
);
INSERT INTO override_test (title, subtitle) VALUES
    ('Hello World', 'hello world'),
    ('hello there', 'Hello There');

-- title has per-field search_tokenizer=simple(lowercase=false) which overrides the index default
-- subtitle has no per-field override, so it uses the index-level unicode_words default
CREATE INDEX idx_override ON override_test USING bm25
    (id,
     (title::pdb.ngram(1, 10, 'prefix_only=true', 'search_tokenizer=simple(lowercase=false)')),
     (subtitle::pdb.ngram(1, 10, 'prefix_only=true'))
    )
    WITH (key_field = 'id', search_tokenizer = 'unicode_words');

-- title: per-field simple(lowercase=false) means "Hello" is NOT lowered at search time
-- -> no match against lowered ngram tokens -> 0 rows
SELECT id, title FROM override_test WHERE title ||| 'Hello' ORDER BY id;

-- title: "hello" is already lowercase -> matches both rows
SELECT id, title FROM override_test WHERE title ||| 'hello' ORDER BY id;

-- subtitle: index-level unicode_words (default lowercase=true) -> "Hello" gets lowered -> matches
SELECT id, subtitle FROM override_test WHERE subtitle ||| 'Hello' ORDER BY id;

-------------------------------------------------------------
-- Test 3: Query-level tokenizer cast overrides index-level
-------------------------------------------------------------

-- On autocomplete table: force edge ngram tokenization at query time
-- "sho" gets ngrammed into s, sh, sho -> matches all 5 titles
SELECT id, title FROM autocomplete WHERE title ||| 'sho'::pdb.ngram(1, 10, 'prefix_only=true') ORDER BY id;

-------------------------------------------------------------
-- Test 4: Parameterized expression as WITH option
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
-- Cleanup
-------------------------------------------------------------

DROP TABLE autocomplete;
DROP TABLE override_test;
DROP TABLE param_test;

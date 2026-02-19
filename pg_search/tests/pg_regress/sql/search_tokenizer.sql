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
    (id, (title::pdb.ngram(1, 10, 'prefix_only=true', 'search_tokenizer=unicode_words')))
    WITH (key_field = 'id');

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

-- Cleanup
DROP TABLE autocomplete;
DROP TABLE autocomplete_plain;
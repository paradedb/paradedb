\i common/common_setup.sql

-- Install citext extension required for all tests in this file
CREATE EXTENSION IF NOT EXISTS citext;

-- ============================================================
-- Test 1: Basic citext column indexing and search
-- ============================================================
CREATE TABLE citext_basic (
    id   INT PRIMARY KEY,
    name CITEXT
);

INSERT INTO citext_basic (id, name) VALUES
    (1, 'Hello World'),
    (2, 'PostgreSQL Database'),
    (3, 'ParadeDB Search'),
    (4, 'Full Text Search'),
    (5, 'Open Source');

CREATE INDEX ON citext_basic
USING bm25 (id, name)
WITH (key_field = 'id');

-- Basic search
SELECT id, name FROM citext_basic WHERE name @@@ 'hello' ORDER BY id;

-- Case-insensitive: same result regardless of query case
SELECT id, name FROM citext_basic WHERE name @@@ 'HELLO' ORDER BY id;
SELECT id, name FROM citext_basic WHERE name @@@ 'Hello' ORDER BY id;

-- Multiple results
SELECT id, name FROM citext_basic WHERE name @@@ 'search' ORDER BY id;

-- No results
SELECT id, name FROM citext_basic WHERE name @@@ 'nonexistent' ORDER BY id;

-- EXPLAIN to verify index is used
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM citext_basic WHERE name @@@ 'hello' ORDER BY id;

DROP TABLE citext_basic;

-- ============================================================
-- Test 2: NULL handling in citext columns
-- ============================================================
CREATE TABLE citext_nulls (
    id      INT PRIMARY KEY,
    content CITEXT
);

INSERT INTO citext_nulls (id, content) VALUES
    (1, 'visible content'),
    (2, NULL),
    (3, 'more content'),
    (4, NULL),
    (5, 'final content');

CREATE INDEX ON citext_nulls
USING bm25 (id, content)
WITH (key_field = 'id');

-- NULLs should not appear in search results
SELECT id, content FROM citext_nulls WHERE content @@@ 'content' ORDER BY id;

-- Verify NULLs are stored correctly (not searched, but fetchable)
SELECT id, content FROM citext_nulls ORDER BY id;

DROP TABLE citext_nulls;

-- ============================================================
-- Test 3: Multiple citext columns in one index
-- ============================================================
CREATE TABLE citext_multi (
    id          INT PRIMARY KEY,
    title       CITEXT,
    description CITEXT
);

INSERT INTO citext_multi (id, title, description) VALUES
    (1, 'Apple',      'A red fruit'),
    (2, 'BANANA',     'A yellow fruit'),
    (3, 'Cherry',     'A small red fruit'),
    (4, 'Dragonfruit','An exotic FRUIT');

CREATE INDEX ON citext_multi
USING bm25 (id, title, description)
WITH (key_field = 'id');

-- Search each column
SELECT id, title FROM citext_multi WHERE title       @@@ 'apple'  ORDER BY id;
SELECT id, title FROM citext_multi WHERE title       @@@ 'APPLE'  ORDER BY id;
SELECT id, title FROM citext_multi WHERE description @@@ 'fruit'  ORDER BY id;
SELECT id, title FROM citext_multi WHERE description @@@ 'FRUIT'  ORDER BY id;

DROP TABLE citext_multi;

-- ============================================================
-- Test 4: Mixed TEXT and CITEXT columns in the same index
-- ============================================================
CREATE TABLE citext_mixed (
    id         INT PRIMARY KEY,
    text_col   TEXT,
    citext_col CITEXT
);

INSERT INTO citext_mixed (id, text_col, citext_col) VALUES
    (1, 'apple orange', 'Banana Cherry'),
    (2, 'Grape Mango',  'pineapple kiwi'),
    (3, 'STRAWBERRY',   'Watermelon');

CREATE INDEX ON citext_mixed
USING bm25 (id, text_col, citext_col)
WITH (key_field = 'id');

-- Both TEXT and CITEXT columns benefit from default tokenizer lowercasing
SELECT id FROM citext_mixed WHERE text_col   @@@ 'apple'     ORDER BY id;
SELECT id FROM citext_mixed WHERE citext_col @@@ 'banana'    ORDER BY id;
SELECT id FROM citext_mixed WHERE citext_col @@@ 'BANANA'    ORDER BY id;
SELECT id FROM citext_mixed WHERE citext_col @@@ 'Banana'    ORDER BY id;

DROP TABLE citext_mixed;

-- ============================================================
-- Test 5: paradedb query builder functions with citext
-- ============================================================
CREATE TABLE citext_queries (
    id      INT PRIMARY KEY,
    content CITEXT
);

INSERT INTO citext_queries (id, content) VALUES
    (1, 'The Quick Brown Fox'),
    (2, 'THE LAZY DOG'),
    (3, 'quick brown fox jumps'),
    (4, 'lazy dog sleeps');

CREATE INDEX ON citext_queries
USING bm25 (id, content)
WITH (key_field = 'id');

-- paradedb.match() with citext
SELECT id FROM citext_queries WHERE content @@@ paradedb.match('content', 'quick') ORDER BY id;
SELECT id FROM citext_queries WHERE content @@@ paradedb.match('content', 'QUICK') ORDER BY id;

-- paradedb.phrase() with citext
SELECT id FROM citext_queries WHERE content @@@ paradedb.phrase('content', ARRAY['quick', 'brown']) ORDER BY id;

-- paradedb.term() does an exact term lookup; terms are lowercased at index time by the
-- tokenizer, so lowercase queries match but uppercase/mixed-case queries do not
SELECT id FROM citext_queries WHERE content @@@ paradedb.term('content', 'quick') ORDER BY id;
SELECT id FROM citext_queries WHERE content @@@ paradedb.term('content', 'QUICK') ORDER BY id;
SELECT id FROM citext_queries WHERE content @@@ paradedb.term('content', 'Quick') ORDER BY id;

DROP TABLE citext_queries;

-- ============================================================
-- Test 6: citext with unicode and special characters
-- ============================================================
CREATE TABLE citext_unicode (
    id   INT PRIMARY KEY,
    name CITEXT
);

INSERT INTO citext_unicode (id, name) VALUES
    (1, 'Ångström'),
    (2, 'Naïve'),
    (3, 'Résumé'),
    (4, 'Café');

CREATE INDEX ON citext_unicode
USING bm25 (id, name)
WITH (key_field = 'id');

SELECT id, name FROM citext_unicode WHERE name @@@ 'naïve'   ORDER BY id;
SELECT id, name FROM citext_unicode WHERE name @@@ 'résumé'  ORDER BY id;
SELECT id, name FROM citext_unicode WHERE name @@@ 'café'    ORDER BY id;

DROP TABLE citext_unicode;

-- ============================================================
-- Test 7: Empty string in citext
-- ============================================================
CREATE TABLE citext_empty (
    id      INT PRIMARY KEY,
    content CITEXT
);

INSERT INTO citext_empty (id, content) VALUES
    (1, ''),
    (2, 'non-empty content'),
    (3, '');

CREATE INDEX ON citext_empty
USING bm25 (id, content)
WITH (key_field = 'id');

SELECT id, content FROM citext_empty WHERE content @@@ 'content' ORDER BY id;

-- Verify stored values come back correctly
SELECT id, content FROM citext_empty ORDER BY id;

DROP TABLE citext_empty;

-- ============================================================
-- Test 8: citext RHS constant in @@@ operator
-- (covers operator.rs: rewrite_rhs_to_search_query_input citext branch)
-- ============================================================
CREATE TABLE citext_rhs (
    id   INT PRIMARY KEY,
    name CITEXT
);

INSERT INTO citext_rhs (id, name) VALUES
    (1, 'Hello World'),
    (2, 'PostgreSQL');

CREATE INDEX ON citext_rhs
USING bm25 (id, name)
WITH (key_field = 'id');

-- RHS is explicitly cast to citext — hits the citext Const branch in operator.rs
SELECT id FROM citext_rhs WHERE name @@@ 'hello'::citext ORDER BY id;
SELECT id FROM citext_rhs WHERE name @@@ 'HELLO'::citext ORDER BY id;

DROP TABLE citext_rhs;

-- ============================================================
-- Test 9: paradedb.term_with_operator with citext value
-- (covers builder_fns/paradedb.rs: term_with_operator citext branch)
-- ============================================================
CREATE TABLE citext_term_op (
    id   INT PRIMARY KEY,
    name CITEXT
);

INSERT INTO citext_term_op (id, name) VALUES
    (1, 'hello'),
    (2, 'world'),
    (3, 'postgres');

CREATE INDEX ON citext_term_op
USING bm25 (id, name)
WITH (key_field = 'id');

-- Passes a citext AnyElement to term_with_operator — hits the citext branch
SELECT id FROM citext_term_op
WHERE id @@@ paradedb.term_with_operator('name', '=', 'hello'::citext)
ORDER BY id;

DROP TABLE citext_term_op;

-- ============================================================
-- Test 10: Columnar execution returning citext fast-field values
-- (covers types_arrow.rs: arrow_array_to_datum citext branch;
--  covers types.rs: try_into_datum citext branch)
-- ============================================================
CREATE TABLE citext_columnar (
    id   INT PRIMARY KEY,
    name CITEXT
);

INSERT INTO citext_columnar (id, name) VALUES
    (1, 'Alpha'),
    (2, 'Beta'),
    (3, 'Gamma');

CREATE INDEX ON citext_columnar
USING bm25 (id, name)
WITH (key_field = 'id');

-- Columnar exec projects the citext fast field directly from the index
SET paradedb.enable_columnar_exec = true;
SELECT id, name FROM citext_columnar WHERE name @@@ 'alpha' ORDER BY id;
SELECT id, name FROM citext_columnar WHERE name @@@ 'beta'  ORDER BY id;
RESET paradedb.enable_columnar_exec;

DROP TABLE citext_columnar;

\i common/common_cleanup.sql

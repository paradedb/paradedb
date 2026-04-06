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
SELECT id, name FROM citext_basic WHERE name ||| 'hello' ORDER BY id;

-- Case-insensitive: same result regardless of query case
SELECT id, name FROM citext_basic WHERE name ||| 'HELLO' ORDER BY id;
SELECT id, name FROM citext_basic WHERE name ||| 'Hello' ORDER BY id;

-- Multiple results
SELECT id, name FROM citext_basic WHERE name ||| 'search' ORDER BY id;

-- No results
SELECT id, name FROM citext_basic WHERE name ||| 'nonexistent' ORDER BY id;

-- EXPLAIN to verify index is used
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM citext_basic WHERE name ||| 'hello' ORDER BY id;

-- Top-K: same score regardless of query case
SELECT id, name, pdb.score(id) > 0 AS has_score
FROM citext_basic
WHERE name ||| 'hello'
ORDER BY pdb.score(id) DESC, id;

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
SELECT id, content FROM citext_nulls WHERE content ||| 'content' ORDER BY id;

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
SELECT id, title FROM citext_multi WHERE title       ||| 'apple'  ORDER BY id;
SELECT id, title FROM citext_multi WHERE title       ||| 'APPLE'  ORDER BY id;
SELECT id, title FROM citext_multi WHERE description ||| 'fruit'  ORDER BY id;
SELECT id, title FROM citext_multi WHERE description ||| 'FRUIT'  ORDER BY id;

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
SELECT id FROM citext_mixed WHERE text_col   ||| 'apple'     ORDER BY id;
SELECT id FROM citext_mixed WHERE citext_col ||| 'banana'    ORDER BY id;
SELECT id FROM citext_mixed WHERE citext_col ||| 'BANANA'    ORDER BY id;
SELECT id FROM citext_mixed WHERE citext_col ||| 'Banana'    ORDER BY id;

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

-- ||| (match) with citext
SELECT id FROM citext_queries WHERE content ||| 'quick' ORDER BY id;
SELECT id FROM citext_queries WHERE content ||| 'QUICK' ORDER BY id;

-- ### (phrase) with citext
SELECT id FROM citext_queries WHERE content ### 'quick brown' ORDER BY id;

-- === (exact term) does an exact term lookup; terms are lowercased at index time by the
-- tokenizer, so lowercase queries match but uppercase/mixed-case queries do not
SELECT id FROM citext_queries WHERE content === 'quick' ORDER BY id;
SELECT id FROM citext_queries WHERE content === 'QUICK' ORDER BY id;
SELECT id FROM citext_queries WHERE content === 'Quick' ORDER BY id;

-- Same BM25 score for 'quick' and 'QUICK' via ||| (both match since tokenizer lowercases)
SELECT id, pdb.score(id) > 0 AS has_score
FROM citext_queries
WHERE content ||| 'quick'
ORDER BY pdb.score(id) DESC, id;

SELECT id, pdb.score(id) > 0 AS has_score
FROM citext_queries
WHERE content ||| 'QUICK'
ORDER BY pdb.score(id) DESC, id;

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

SELECT id, name FROM citext_unicode WHERE name ||| 'naïve'   ORDER BY id;
SELECT id, name FROM citext_unicode WHERE name ||| 'résumé'  ORDER BY id;
SELECT id, name FROM citext_unicode WHERE name ||| 'café'    ORDER BY id;

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

SELECT id, content FROM citext_empty WHERE content ||| 'content' ORDER BY id;

-- Verify stored values come back correctly
SELECT id, content FROM citext_empty ORDER BY id;

DROP TABLE citext_empty;

-- ============================================================
-- Test 8: Aggregate scan over a citext column
-- (covers types.rs: try_into_datum citext branch via aggregatescan)
-- ============================================================
CREATE TABLE citext_agg (
    id      INT PRIMARY KEY,
    category CITEXT,
    value    INT
);

INSERT INTO citext_agg (id, category, value) VALUES
    (1, 'Alpha', 10),
    (2, 'Beta',  20),
    (3, 'Alpha', 30),
    (4, 'Gamma', 40),
    (5, 'Beta',  50);

CREATE INDEX ON citext_agg
USING bm25 (id, category, value)
WITH (key_field = 'id');

-- GROUP BY on citext column — aggregatescan calls try_into_datum with citext OID
SELECT category, COUNT(*) FROM citext_agg
WHERE category ||| 'alpha beta gamma'
GROUP BY category
ORDER BY category;

SELECT category, SUM(value) FROM citext_agg
WHERE category ||| 'alpha beta'
GROUP BY category
ORDER BY category;

DROP TABLE citext_agg;

-- ============================================================
-- Test 9: citext RHS constant in @@@ operator
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

-- Case-insensitive match via v2 operator
SELECT id FROM citext_rhs WHERE name ||| 'hello' ORDER BY id;
SELECT id FROM citext_rhs WHERE name ||| 'HELLO' ORDER BY id;

DROP TABLE citext_rhs;

-- ============================================================
-- Test 10: paradedb.term_with_operator with citext value
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
-- Test 11: Columnar execution returning citext fast-field values
-- (covers types_arrow.rs: arrow_array_to_datum citext branch)
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
SELECT id, name FROM citext_columnar WHERE name ||| 'alpha' ORDER BY id;
SELECT id, name FROM citext_columnar WHERE name ||| 'beta'  ORDER BY id;
RESET paradedb.enable_columnar_exec;

DROP TABLE citext_columnar;

-- ============================================================
-- Test 12: Term matching and Top-K ordering with citext case variants
-- ============================================================
CREATE TABLE citext_topk (
    id      INT PRIMARY KEY,
    content CITEXT
);

INSERT INTO citext_topk (id, content) VALUES
    (1, 'quick quick quick'),              -- reference row: all lowercase
    -- three rows with identical token content, just different casing — must score identically
    (2, 'quick quick quick quick fox'),    -- lowercase
    (3, 'QUICK QUICK QUICK QUICK FOX'),    -- uppercase
    (4, 'QUICK QUICK qUicK QUicK FOX'),   -- wildly mixed-case
    -- phrase-match target and non-match
    (5, 'Quick Brown Fox'),               -- mixed-case, for phrase test
    (6, 'brown fox');                     -- no 'quick' → not returned

CREATE INDEX ON citext_topk
USING bm25 (id, content)
WITH (key_field = 'id');

-- ||| (match): lowercase and UPPERCASE queries must return the same rows in the same Top-K order
-- rows 2, 3, 4 have identical token content at different case — their scores must be equal
SELECT id, round(pdb.score(id)::numeric, 4) AS score, content
FROM citext_topk
WHERE content ||| 'quick'
ORDER BY pdb.score(id) DESC, id;

SELECT id, round(pdb.score(id)::numeric, 4) AS score, content
FROM citext_topk
WHERE content ||| 'QUICK'
ORDER BY pdb.score(id) DESC, id;

-- === (exact term): only lowercase matches (terms are stored lowercased at index time)
SELECT id, content FROM citext_topk WHERE content === 'quick' ORDER BY id;
SELECT id, content FROM citext_topk WHERE content === 'QUICK' ORDER BY id;  -- expects 0 rows

-- paradedb.term() with 'QUICK': raw term lookup — 0 rows since tokens are stored lowercased
SELECT id, content FROM citext_topk
WHERE content ||| paradedb.term('content', 'QUICK')
ORDER BY id;

-- ### (phrase): case-insensitive phrase matching
SELECT id, content FROM citext_topk WHERE content ### 'quick brown' ORDER BY id;
SELECT id, content FROM citext_topk WHERE content ### 'QUICK BROWN' ORDER BY id;

DROP TABLE citext_topk;

-- ============================================================
-- Test 13: Prepared statements with citext parameters under generic plan
-- (covers build_text_funcexpr citext branch in exec_rewrite)
-- ============================================================
CREATE TABLE citext_prepared (
    id      INT PRIMARY KEY,
    content CITEXT
);

INSERT INTO citext_prepared (id, content) VALUES
    (1, 'Hello World'),
    (2, 'PostgreSQL Database'),
    (3, 'ParadeDB Search');

CREATE INDEX ON citext_prepared
USING bm25 (id, content)
WITH (key_field = 'id');

SET plan_cache_mode = force_generic_plan;

-- &&& with citext parameter
PREPARE citext_and(citext) AS
SELECT id FROM citext_prepared WHERE content &&& $1 ORDER BY id;
EXECUTE citext_and('hello');
DEALLOCATE citext_and;

-- ||| with citext parameter
PREPARE citext_or(citext) AS
SELECT id FROM citext_prepared WHERE content ||| $1 ORDER BY id;
EXECUTE citext_or('hello');
DEALLOCATE citext_or;

-- ### with citext parameter
PREPARE citext_phrase(citext) AS
SELECT id FROM citext_prepared WHERE content ### $1 ORDER BY id;
EXECUTE citext_phrase('hello world');
DEALLOCATE citext_phrase;

-- === with citext parameter
PREPARE citext_term(citext) AS
SELECT id FROM citext_prepared WHERE content === $1 ORDER BY id;
EXECUTE citext_term('hello');
DEALLOCATE citext_term;

-- @@@ with citext parameter
PREPARE citext_parse(citext) AS
SELECT id FROM citext_prepared WHERE content @@@ $1 ORDER BY id;
EXECUTE citext_parse('hello');
DEALLOCATE citext_parse;

RESET plan_cache_mode;

DROP TABLE citext_prepared;

\i common/common_cleanup.sql

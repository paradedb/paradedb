-- A `pdb.expand` subquery makes the query parser expand each word and each quoted phrase of its
-- query string.

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS parse_products, parse_notes, parse_synonyms;

CREATE TABLE parse_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO parse_synonyms VALUES
    ('sofa, couch, divan'),
    ('sneakers => running shoes'),
    ('trail shoes, hikers');

CREATE TABLE parse_products (id int PRIMARY KEY, description text);
INSERT INTO parse_products VALUES
    (1, 'red couch'),
    (2, 'red leather sofa'),
    (3, 'oak divan'),
    (4, 'blue running shoes'),
    (5, 'blue shoes for running'),
    (6, 'blue sneakers'),
    (7, 'green hikers'),
    (8, 'trail map'),
    (9, 'red chair'),
    (10, 'soda can'),
    (11, 'night coach');
CREATE INDEX parse_products_idx ON parse_products USING bm25 (id, description);

-- The rules table needs no index. The subquery reads the rules, and the parsed query expands
-- with them when the query runs.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms) ORDER BY id;

-- With a synonym index on the rule column, the query reads the index.
CREATE INDEX parse_synonyms_idx ON parse_synonyms USING paradedb (rule);
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms) ORDER BY id;

-- The operators of the query string apply to the group of a word, so `-sofa` also removes the
-- rows with `couch`.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('red AND sofa', rule) FROM parse_synonyms);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('red AND sofa', rule) FROM parse_synonyms) ORDER BY id;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('red -sofa', rule) FROM parse_synonyms);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('red -sofa', rule) FROM parse_synonyms) ORDER BY id;

-- A quoted phrase expands as the phrase operator does, and it keeps its slop.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('"sofa red"~2', rule) FROM parse_synonyms);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('"sofa red"~2', rule) FROM parse_synonyms) ORDER BY id;

-- A stop word leaves a gap in a quoted phrase, and each phrase keeps the gap. No synonym index
-- has the tokenizer of this field, so the subquery runs as written.
CREATE TABLE parse_notes (id int PRIMARY KEY, note text);
INSERT INTO parse_notes VALUES
    (1, 'sofa of leather'),
    (2, 'couch of leather'),
    (3, 'couch leather');
CREATE INDEX parse_notes_idx ON parse_notes
    USING bm25 (id, (note::pdb.simple('stopwords_language=English')));
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_notes
WHERE note @@@ (SELECT pdb.expand('"sofa of leather"', rule) FROM parse_synonyms);
SELECT id FROM parse_notes
WHERE note @@@ (SELECT pdb.expand('"sofa of leather"', rule) FROM parse_synonyms) ORDER BY id;
DROP TABLE parse_notes;

-- A directed rule replaces the typed word with its phrase.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sneakers', rule) FROM parse_synonyms);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sneakers', rule) FROM parse_synonyms) ORDER BY id;

-- The parser makes one query for each word, so a rule on `trail shoes` needs the quotes.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('trail shoes', rule) FROM parse_synonyms);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('trail shoes', rule) FROM parse_synonyms) ORDER BY id;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('"trail shoes"', rule) FROM parse_synonyms);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('"trail shoes"', rule) FROM parse_synonyms) ORDER BY id;

-- A fuzzy query keeps the typed word fuzzy, so it finds `soda`. It matches the alternatives
-- exactly, so `couch` does not find `coach`.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms)::pdb.fuzzy(1);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms)::pdb.fuzzy(1)
ORDER BY id;

-- A generic plan has no text, so it gives the parameter to the index when the query runs.
PREPARE parse_sofa(text) AS SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand($1, rule) FROM parse_synonyms) ORDER BY id;
SET plan_cache_mode = force_generic_plan;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE parse_sofa('sofa');
EXECUTE parse_sofa('sofa');
RESET plan_cache_mode;
DEALLOCATE parse_sofa;

-- The rules hold words, so the left side has to be a text field.
SELECT id FROM parse_products
WHERE id @@@ (SELECT pdb.expand('description:sofa', rule) FROM parse_synonyms);

DROP TABLE parse_products, parse_synonyms;

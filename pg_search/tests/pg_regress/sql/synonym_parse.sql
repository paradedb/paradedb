-- A `pdb.expand` subquery makes the query parser expand each word and each quoted phrase of its
-- query string.

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS parse_products, parse_notes, parse_stems, parse_grams, parse_synonyms;

CREATE TABLE parse_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO parse_synonyms VALUES
    ('sofa, couch, divan'),
    ('sneakers => running shoes'),
    ('trail shoes, hikers');

CREATE TABLE parse_products (id int PRIMARY KEY, description text, category text, metadata jsonb);
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
    (11, 'night coach'),
    (12, 'running shoes sale'),
    (13, 'hikers sale');
INSERT INTO parse_products VALUES
    (14, 'oak table', 'sofa', '{"seat": "sofa"}'),
    (15, 'pine table', 'couch', '{"seat": "couch"}');
CREATE INDEX parse_products_idx ON parse_products
    USING bm25 (id, description, category, metadata);

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
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('red -sofa', rule) FROM parse_synonyms) ORDER BY id;

-- A word in a group or with a boost expands too.
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('(sofa^2 OR chair) AND red', rule) FROM parse_synonyms)
ORDER BY id;

-- A quoted phrase expands as the phrase operator does, and it keeps its slop.
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('"sofa red"~2', rule) FROM parse_synonyms) ORDER BY id;

-- An alternative can have more or fewer words than the words it replaces. The words after it
-- keep their distance to it.
SELECT id FROM parse_products
WHERE description @@@ (
    SELECT pdb.expand('"sneakers sale" OR "trail shoes sale"', rule) FROM parse_synonyms
) ORDER BY id;

-- A stop word leaves a gap in a quoted phrase, and each phrase keeps the gap. No synonym index
-- has the tokenizer of this field, so the subquery runs as written.
CREATE TABLE parse_notes (id int PRIMARY KEY, note text);
INSERT INTO parse_notes VALUES
    (1, 'sofa of leather'),
    (2, 'couch of leather'),
    (3, 'couch leather');
CREATE INDEX parse_notes_idx ON parse_notes
    USING bm25 (id, (note::pdb.simple('stopwords_language=English')));
SELECT id FROM parse_notes
WHERE note @@@ (SELECT pdb.expand('"sofa of leather"', rule) FROM parse_synonyms) ORDER BY id;
DROP TABLE parse_notes;

-- A directed rule replaces the typed word with its phrase.
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sneakers', rule) FROM parse_synonyms) ORDER BY id;

-- The parser makes one query for each word, so a rule on `trail shoes` needs the quotes.
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('trail shoes', rule) FROM parse_synonyms) ORDER BY id;
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('"trail shoes"', rule) FROM parse_synonyms) ORDER BY id;

-- A word for another field stays as typed, so `category:sofa` does not find the category `couch`.
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('category:sofa', rule) FROM parse_synonyms) ORDER BY id;

-- A JSON field expands at each of its paths, with the path on the left side or in the query
-- string.
SELECT id FROM parse_products
WHERE metadata->>'seat' @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms) ORDER BY id;
SELECT id FROM parse_products
WHERE metadata @@@ (SELECT pdb.expand('seat:sofa', rule) FROM parse_synonyms) ORDER BY id;

-- A fuzzy query keeps the typed word fuzzy, so it finds `soda`. It matches the alternatives
-- exactly, so `couch` does not find `coach`.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms)::pdb.fuzzy(1);
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms)::pdb.fuzzy(1)
ORDER BY id;

-- The setting caps the alternatives of a word, and the typed word stays.
SET paradedb.synonyms_max_alternatives = 2;
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms) ORDER BY id;
RESET paradedb.synonyms_max_alternatives;

-- A top-k by score of one word stays serial, since Block-WAND prunes one term. A word that a rule
-- turns into a group can take workers.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('chair', rule) FROM parse_synonyms)
ORDER BY pdb.score(id) DESC, id LIMIT 2;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms)
ORDER BY pdb.score(id) DESC, id LIMIT 2;

-- A generic plan has no text, so it gives the parameter to the index when the query runs.
PREPARE parse_sofa(text) AS SELECT id FROM parse_products
WHERE description @@@ (SELECT pdb.expand($1, rule) FROM parse_synonyms) ORDER BY id;
SET plan_cache_mode = force_generic_plan;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE parse_sofa('sofa');
EXECUTE parse_sofa('sofa');
RESET plan_cache_mode;
DEALLOCATE parse_sofa;

-- The parser does not use the `search_tokenizer`, so the query reads the synonym index that has
-- the tokenizer of the field.
CREATE TABLE parse_stems (id int PRIMARY KEY, title text);
INSERT INTO parse_stems VALUES (1, 'running shoes'), (2, 'sneakers');
CREATE INDEX parse_stems_idx ON parse_stems
    USING bm25 (id, (title::pdb.simple('stemmer=english'))) WITH (search_tokenizer = 'simple');
CREATE INDEX parse_synonyms_stem_idx ON parse_synonyms
    USING paradedb ((rule::pdb.simple('stemmer=english')));
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM parse_stems
WHERE title @@@ (SELECT pdb.expand('sneakers', rule) FROM parse_synonyms);
SELECT id FROM parse_stems
WHERE title @@@ (SELECT pdb.expand('sneakers', rule) FROM parse_synonyms) ORDER BY id;
DROP INDEX parse_synonyms_stem_idx;
DROP TABLE parse_stems;

-- An alternative of several tokens is a phrase, so the field has to keep positions.
CREATE TABLE parse_grams (id int PRIMARY KEY, title text);
INSERT INTO parse_grams VALUES (1, 'sofa');
CREATE INDEX parse_grams_idx ON parse_grams USING bm25 (id, (title::pdb.ngram(3, 3)));
SELECT id FROM parse_grams
WHERE title @@@ (SELECT pdb.expand('sofa', rule) FROM parse_synonyms);
DROP TABLE parse_grams;

-- The rules hold words, so the left side has to be a text field.
SELECT id FROM parse_products
WHERE id @@@ (SELECT pdb.expand('description:sofa', rule) FROM parse_synonyms);

DROP TABLE parse_products, parse_synonyms;

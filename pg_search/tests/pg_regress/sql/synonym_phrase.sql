-- A `pdb.expand` subquery makes a phrase query search one phrase for each choice of alternatives.

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS phrase_products, phrase_synonyms;

CREATE TABLE phrase_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO phrase_synonyms VALUES
    ('sofa, couch, divan'),
    ('bed, cot'),
    ('sneakers => running shoes');

CREATE TABLE phrase_products (id int PRIMARY KEY, description text);
INSERT INTO phrase_products VALUES
    (1, 'red couch'),
    (2, 'red leather sofa'),
    (3, 'divan in red'),
    (4, 'sofa bed'),
    (5, 'couch cot'),
    (6, 'blue running shoes'),
    (7, 'blue shoes for running'),
    (8, 'blue sneakers'),
    (9, 'couch'),
    (10, 'sofa cot');
CREATE INDEX phrase_products_idx ON phrase_products USING bm25 (id, description);

-- The rules table needs no index. The subquery reads the rules, and the phrase expands with them.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('red sofa', rule) FROM phrase_synonyms);
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('red sofa', rule) FROM phrase_synonyms) ORDER BY id;

-- With a synonym index on the rule column, the query reads the index.
CREATE INDEX phrase_synonyms_idx ON phrase_synonyms USING paradedb (rule);
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('red sofa', rule) FROM phrase_synonyms);
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('red sofa', rule) FROM phrase_synonyms) ORDER BY id;

-- Each word with a rule multiplies the phrases, so `sofa bed` is six of them.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('sofa bed', rule) FROM phrase_synonyms);
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('sofa bed', rule) FROM phrase_synonyms) ORDER BY id;

-- The cap keeps the typed phrase and the first phrases after it. `sofa cot` stays, and `couch cot`
-- does not.
SET paradedb.synonyms_max_alternatives = 3;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('sofa bed', rule) FROM phrase_synonyms);
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('sofa bed', rule) FROM phrase_synonyms) ORDER BY id;
RESET paradedb.synonyms_max_alternatives;

-- A multi-word alternative adds all its words to the phrase, and a directed rule drops the typed
-- word.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('blue sneakers', rule) FROM phrase_synonyms);
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('blue sneakers', rule) FROM phrase_synonyms) ORDER BY id;

-- The slop applies to each phrase.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('sofa red', rule) FROM phrase_synonyms)::pdb.slop(2);
SELECT id FROM phrase_products
WHERE description ### (SELECT pdb.expand('sofa red', rule) FROM phrase_synonyms)::pdb.slop(2)
ORDER BY id;

-- A phrase of one word scores its alternatives as one term, the same as a match.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id) FROM phrase_products
WHERE description ### (SELECT pdb.expand('sofa', rule) FROM phrase_synonyms)
ORDER BY pdb.score(id) DESC, id;
SELECT id, pdb.score(id) FROM phrase_products
WHERE description ### (SELECT pdb.expand('sofa', rule) FROM phrase_synonyms)
ORDER BY pdb.score(id) DESC, id;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id) FROM phrase_products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM phrase_synonyms)
ORDER BY pdb.score(id) DESC, id;
SELECT id, pdb.score(id) FROM phrase_products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM phrase_synonyms)
ORDER BY pdb.score(id) DESC, id;

DROP TABLE phrase_products, phrase_synonyms;

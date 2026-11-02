\i common/common_setup.sql

-- Query-time synonyms. The rules live in paradedb.synonym_sets, in the Solr format, and a set
-- attaches to an index or to a field. Expansion happens when a match query is built, so a change
-- to the table is live on the next query with no reindex.

INSERT INTO paradedb.synonym_sets (name, rules) VALUES
    ('shop', ARRAY[
        '# comments and blank lines are skipped',
        '',
        'couch, sofa, divan',
        'sneakers => running shoes',
        'tv, television',
        'red, crimson'
    ]),
    ('colors', ARRAY['red, crimson, scarlet']);

SELECT name, version, rules FROM paradedb.synonym_sets ORDER BY name;

-- 'none' is the value that turns expansion off, so it cannot name a set.
INSERT INTO paradedb.synonym_sets (name, rules) VALUES ('none', ARRAY['a, b']);

DROP TABLE IF EXISTS products;
CREATE TABLE products (
    id serial8 NOT NULL PRIMARY KEY,
    description text,
    attrs jsonb
);
INSERT INTO products (description, attrs) VALUES
    ('A red sofa for the living room', '{"color": "red"}'),
    ('Leather couch with cushions', '{"color": "brown"}'),
    ('Antique divan', '{"color": "crimson"}'),
    ('Running shoes for marathons', '{"color": "white"}'),
    ('Sneakers for the gym', '{"color": "black"}'),
    ('Big television', '{"color": "black"}'),
    ('A sofa and a couch, both red', '{"color": "red"}'),
    ('Sofas on sale', '{"color": "blue"}'),
    ('Couches on sale', '{"color": "blue"}');

-------------------------------------------------------------
-- Index level: WITH (synonyms = ...)
-------------------------------------------------------------

CREATE INDEX idx_products ON products USING bm25 (id, description, attrs)
    WITH (synonyms = 'shop');

-- Equivalents: every term of the group finds the whole group.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, description FROM products WHERE description ||| 'sofa' ORDER BY id;
SELECT id, description FROM products WHERE description ||| 'sofa' ORDER BY id;
SELECT id, description FROM products WHERE description ||| 'divan' ORDER BY id;

-- Directed: the left side is replaced, here by a two-word phrase.
SELECT id, description FROM products WHERE description ||| 'sneakers' ORDER BY id;
-- The other direction does not expand.
SELECT id, description FROM products WHERE description ||| 'running shoes' ORDER BY id;

-- Scoring: the alternatives of one input share a DisjunctionMax, so the row with both "sofa" and
-- "couch" scores once for that input, with the best alternative, and not twice.
SELECT id, description, paradedb.score(id) FROM products WHERE description ||| 'sofa' ORDER BY id;
SELECT id, description, paradedb.score(id) FROM products WHERE description ||| 'couch' ORDER BY id;

-- Conjunction: every group must match.
SELECT id, description FROM products WHERE description &&& 'red sofa' ORDER BY id;
SELECT id, description FROM products WHERE description &&& 'leather couch' ORDER BY id;

-- Fuzzy: the typed token stays fuzzy (sofa ~ sofas), the alternatives are exact
-- (couch does not reach couches).
SELECT id, description FROM products WHERE description ||| 'sofa'::pdb.fuzzy(1) ORDER BY id;

-- A tokenizer cast on the query gives exact tokens, with no expansion.
SELECT id, description FROM products WHERE description ||| 'sofa'::pdb.simple ORDER BY id;

-- Term and phrase queries are exact by definition.
SELECT id, description FROM products WHERE description === 'sofa' ORDER BY id;
SELECT id, description FROM products WHERE description ### 'red sofa' ORDER BY id;

-- JSON fields expand with the index set too.
SELECT id, attrs FROM products WHERE attrs->'color' ||| 'red' ORDER BY id;

-------------------------------------------------------------
-- The debug surface
-------------------------------------------------------------

SELECT * FROM paradedb.synonym_expansion('shop', paradedb.tokenizer('default'), 'red sofa and sneakers');
SELECT * FROM paradedb.synonym_expansion('shop', paradedb.tokenizer('default'), 'running shoes');

-- A right side that the analyzer removes, here a stopword, keeps the typed term.
INSERT INTO paradedb.synonym_sets (name, rules) VALUES ('stop', ARRAY['sofa => the', 'couch => the settee']);
SELECT * FROM paradedb.synonym_expansion('stop', paradedb.tokenizer('default', stopwords_language => 'English'), 'sofa couch');

-------------------------------------------------------------
-- Reload: an UPDATE is visible to its own transaction before commit, and to every query after.
-------------------------------------------------------------

BEGIN;
UPDATE paradedb.synonym_sets SET rules = ARRAY['sofa, settee'] WHERE name = 'shop';
SELECT id, description FROM products WHERE description ||| 'sofa' ORDER BY id;
ROLLBACK;

-- Rolled back: the old rules apply again.
SELECT id, description FROM products WHERE description ||| 'sofa' ORDER BY id;

UPDATE paradedb.synonym_sets SET rules = array_append(rules, 'sofa, loveseat') WHERE name = 'shop';
SELECT name, version FROM paradedb.synonym_sets WHERE name = 'shop';
INSERT INTO products (description, attrs) VALUES ('Loveseat for two', '{"color": "green"}');
SELECT id, description FROM products WHERE description ||| 'sofa' ORDER BY id;

-------------------------------------------------------------
-- ALTER INDEX changes the set with no reindex.
-------------------------------------------------------------

ALTER INDEX idx_products SET (synonyms = 'colors');
SELECT id, description FROM products WHERE description ||| 'sofa' ORDER BY id;
SELECT id, description FROM products WHERE description ||| 'crimson' ORDER BY id;

ALTER INDEX idx_products RESET (synonyms);
SELECT id, description FROM products WHERE description ||| 'crimson' ORDER BY id;

-- A missing set fails at query time, with its name.
ALTER INDEX idx_products SET (synonyms = 'nope');
SELECT id, description FROM products WHERE description ||| 'sofa' ORDER BY id;
ALTER INDEX idx_products SET (synonyms = 'shop');

-- An invalid rule names the set and the rule.
INSERT INTO paradedb.synonym_sets (name, rules) VALUES ('bad', ARRAY['a => b => c']);
ALTER INDEX idx_products SET (synonyms = 'bad');
SELECT id, description FROM products WHERE description ||| 'sofa' ORDER BY id;
ALTER INDEX idx_products SET (synonyms = 'shop');

-------------------------------------------------------------
-- Parallel workers expand the same way as the leader.
-------------------------------------------------------------

SET debug_parallel_query = on;
SET max_parallel_workers_per_gather = 2;
SET parallel_tuple_cost = 0;
SET parallel_setup_cost = 0;
SET min_parallel_table_scan_size = 0;

SELECT id, description FROM products WHERE description ||| 'sofa' ORDER BY id;
SELECT id, description FROM products WHERE description &&& 'red sofa' ORDER BY id;

RESET debug_parallel_query;
RESET max_parallel_workers_per_gather;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
RESET min_parallel_table_scan_size;

DROP TABLE products;

-------------------------------------------------------------
-- Field level: synonyms=... on the tokenizer cast overrides the index option.
-------------------------------------------------------------

DROP TABLE IF EXISTS fields;
CREATE TABLE fields (
    id serial8 NOT NULL PRIMARY KEY,
    title text,
    body text,
    notes text
);
INSERT INTO fields (title, body, notes) VALUES
    ('red sofa', 'crimson couch', 'scarlet divan');

CREATE INDEX idx_fields ON fields USING bm25 (
    id,
    (title::pdb.simple('synonyms=colors')),
    (body::pdb.simple('synonyms=none')),
    notes
) WITH (synonyms = 'shop');

-- title: the colors set, so crimson finds red and sofa does not find couch.
SELECT id FROM fields WHERE title ||| 'crimson' ORDER BY id;
SELECT id FROM fields WHERE title ||| 'couch' ORDER BY id;
-- body: no expansion at all.
SELECT id FROM fields WHERE body ||| 'red' ORDER BY id;
SELECT id FROM fields WHERE body ||| 'crimson' ORDER BY id;
-- notes: the index set.
SELECT id FROM fields WHERE notes ||| 'sofa' ORDER BY id;
SELECT id FROM fields WHERE notes ||| 'red' ORDER BY id;

DROP TABLE fields;

-------------------------------------------------------------
-- The rules go through the field's analyzer, so a stemmer applies to both sides.
-------------------------------------------------------------

DROP TABLE IF EXISTS stemmed;
CREATE TABLE stemmed (
    id serial8 NOT NULL PRIMARY KEY,
    description text
);
INSERT INTO stemmed (description) VALUES
    ('running shoes for marathons'),
    ('a runner shoe'),
    ('sneaker for the gym');

CREATE INDEX idx_stemmed ON stemmed USING bm25 (id, (description::pdb.simple('stemmer=english')))
    WITH (synonyms = 'shop');

-- "sneakers" stems to "sneaker", and "running shoes" to the phrase "run shoe".
SELECT * FROM paradedb.synonym_expansion('shop', paradedb.tokenizer('default', stemmer => 'English'), 'sneaker');
SELECT id, description FROM stemmed WHERE description ||| 'sneaker' ORDER BY id;
SELECT id, description FROM stemmed WHERE description ||| 'sneakers' ORDER BY id;

DROP TABLE stemmed;

DELETE FROM paradedb.synonym_sets;

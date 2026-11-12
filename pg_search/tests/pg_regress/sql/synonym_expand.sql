-- A `pdb.expand` subquery on a rules table makes a match query expand with the rules it reads.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 2;
DROP VIEW IF EXISTS sofa_products;
DROP TABLE IF EXISTS products, en_products, shop_synonyms, text_rules CASCADE;
DROP ROLE IF EXISTS synonym_expand_reader;

CREATE TABLE shop_synonyms (rule pdb.synonym_rule NOT NULL, shop text NOT NULL DEFAULT 'main');
INSERT INTO shop_synonyms VALUES
    ('sofa, couch, divan'),
    ('sneakers => running shoes'),
    ('tv, television');
INSERT INTO shop_synonyms VALUES ('sofa, settee', 'outlet');

CREATE TABLE products (id int PRIMARY KEY, description text);
INSERT INTO products VALUES
    (1, 'red couch'),
    (2, 'leather sofa'),
    (3, 'oak divan'),
    (4, 'running shoes for trails'),
    (5, 'shoes for running'),
    (6, 'blue sneakers'),
    (7, 'television stand'),
    (8, 'sofa and couch set'),
    (9, 'velvet settee');
CREATE INDEX products_idx ON products USING bm25 (id, description);

-- The rules table needs no index. The subquery reads the rules, and the match expands with them.
-- A planner warning says why no index is read.
EXPLAIN (COSTS OFF)
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'main');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'main')
ORDER BY id;
SELECT id FROM products
WHERE description &&& (SELECT pdb.expand('red sofa', rule) FROM shop_synonyms WHERE shop = 'main')
ORDER BY id;

-- A multi-word alternative is a phrase, and a directed rule drops the typed word.
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sneakers', rule) FROM shop_synonyms) ORDER BY id;

-- The subquery picks the rules, so one table holds the rules of several shops.
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'outlet')
ORDER BY id;

-- A subquery that finds no rule leaves the typed words.
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'none')
ORDER BY id;
PREPARE no_rules(text) AS SELECT id FROM products
WHERE description ||| (SELECT pdb.expand($1, rule) FROM shop_synonyms WHERE shop = 'none')
ORDER BY id;
SET plan_cache_mode = force_generic_plan;
EXECUTE no_rules('sofa');
RESET plan_cache_mode;
DEALLOCATE no_rules;

-- A parallel worker gets the rules that the leader read.
SET debug_parallel_query = on;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'main')
ORDER BY id;
RESET debug_parallel_query;

-- The setting caps the alternatives, and the typed word stays.
SET paradedb.synonyms_max_alternatives = 2;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('divan', rule) FROM shop_synonyms WHERE shop = 'main')
ORDER BY id;
RESET paradedb.synonyms_max_alternatives;

-- The rules are compiled with the tokenizer of the field, so a stemmed field expands a stemmed word.
CREATE TABLE en_products (id int PRIMARY KEY, description text);
INSERT INTO en_products VALUES (1, 'two couches'), (2, 'a sofa'), (3, 'a divan');
CREATE INDEX en_products_idx ON en_products USING bm25 (
    id, (description::pdb.simple('stemmer=english'))
);
SELECT id FROM en_products
WHERE description ||| (SELECT pdb.expand('sofas', rule) FROM shop_synonyms WHERE shop = 'main')
ORDER BY id;

-- Postgres checks the rights on the rules table, and row security picks the rules of the role.
CREATE ROLE synonym_expand_reader;
GRANT SELECT ON products TO synonym_expand_reader;
SET ROLE synonym_expand_reader;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
RESET ROLE;
GRANT SELECT ON shop_synonyms TO synonym_expand_reader;
ALTER TABLE shop_synonyms ENABLE ROW LEVEL SECURITY;
CREATE POLICY outlet_rules ON shop_synonyms TO synonym_expand_reader USING (shop = 'outlet');
SET ROLE synonym_expand_reader;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
RESET ROLE;

-- With a synonym index on the rule column, a subquery that reads the whole table reads the index.
DELETE FROM shop_synonyms WHERE shop = 'outlet';
CREATE INDEX shop_synonyms_idx ON shop_synonyms USING paradedb (rule);
EXPLAIN (COSTS OFF)
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms);

-- A group scores as one term, and the index gives the scores that the rules give.
SELECT id, round(pdb.score(id)::numeric, 4) AS score FROM products
WHERE description ||| (SELECT pdb.expand('couch', rule) FROM shop_synonyms)
ORDER BY score DESC, id;
SELECT id, round(pdb.score(id)::numeric, 4) AS score FROM products
WHERE description ||| (SELECT pdb.expand('couch', rule) FROM shop_synonyms WHERE shop = 'main')
ORDER BY score DESC, id;

-- A fuzzy match keeps the typed word fuzzy and matches the alternatives exactly.
INSERT INTO products VALUES (10, 'two sofas');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'main')::pdb.fuzzy(1)
ORDER BY id;
DELETE FROM products WHERE id = 10;

-- A prepared statement reads the index with both kinds of plan. A generic plan has no text, so
-- it gives the parameter to the index when the query runs.
PREPARE sofa_query(text) AS SELECT id FROM products
WHERE description ||| (SELECT pdb.expand($1, rule) FROM shop_synonyms) ORDER BY id;
SET plan_cache_mode = force_custom_plan;
EXPLAIN (COSTS OFF) EXECUTE sofa_query('sofa');
EXECUTE sofa_query('sofa');
SET plan_cache_mode = force_generic_plan;
EXPLAIN (COSTS OFF) EXECUTE sofa_query('sofa');
EXECUTE sofa_query('sofa');
RESET plan_cache_mode;
DEALLOCATE sofa_query;

-- A partial synonym index holds the rules of its predicate. A subquery with that predicate reads
-- it, and a subquery on the whole table does not.
DROP INDEX shop_synonyms_idx;
INSERT INTO shop_synonyms VALUES ('sofa, settee', 'outlet');
CREATE INDEX outlet_synonyms_idx ON shop_synonyms USING paradedb (rule) WHERE shop = 'outlet';
EXPLAIN (COSTS OFF)
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'outlet');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'outlet')
ORDER BY id;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
-- A generic plan has no value for a parameter in the filter, so it can't pick a partial index.
PREPARE outlet_query(text) AS SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = $1)
ORDER BY id;
SET plan_cache_mode = force_generic_plan;
EXECUTE outlet_query('outlet');
RESET plan_cache_mode;
DEALLOCATE outlet_query;
DROP INDEX outlet_synonyms_idx;
DELETE FROM shop_synonyms WHERE shop = 'outlet';
CREATE INDEX shop_synonyms_idx ON shop_synonyms USING paradedb (rule);

-- A synonym index that is not valid is not read.
UPDATE pg_index SET indisvalid = false WHERE indexrelid = 'shop_synonyms_idx'::regclass;
EXPLAIN (COSTS OFF)
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms);
UPDATE pg_index SET indisvalid = true WHERE indexrelid = 'shop_synonyms_idx'::regclass;

-- A subquery with a filter that no index has still reads the rules itself. The warning says why,
-- and `paradedb.planner_warnings` makes it an error.
EXPLAIN (COSTS OFF)
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'main');
SET paradedb.planner_warnings = 'error';
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'main');
RESET paradedb.planner_warnings;

-- A role under row security reads the rules itself, because the index holds every rule.
INSERT INTO shop_synonyms VALUES ('sofa, settee', 'outlet');
SET ROLE synonym_expand_reader;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
RESET ROLE;
DELETE FROM shop_synonyms WHERE shop = 'outlet';
DROP POLICY outlet_rules ON shop_synonyms;
ALTER TABLE shop_synonyms DISABLE ROW LEVEL SECURITY;
REVOKE SELECT ON products, shop_synonyms FROM synonym_expand_reader;
DROP ROLE synonym_expand_reader;

-- A synonym index with another tokenizer than the field is not read, and the query still expands.
EXPLAIN (COSTS OFF)
SELECT id FROM en_products
WHERE description ||| (SELECT pdb.expand('sofas', rule) FROM shop_synonyms);
SELECT id FROM en_products
WHERE description ||| (SELECT pdb.expand('sofas', rule) FROM shop_synonyms) ORDER BY id;

-- A second synonym index with the tokenizer of the field is the one that the query reads.
CREATE INDEX shop_synonyms_en_idx ON shop_synonyms
    USING paradedb ((rule::pdb.simple('stemmer=english')));
EXPLAIN (COSTS OFF)
SELECT id FROM en_products
WHERE description ||| (SELECT pdb.expand('sofas', rule) FROM shop_synonyms);
SELECT id FROM en_products
WHERE description ||| (SELECT pdb.expand('sofas', rule) FROM shop_synonyms) ORDER BY id;
DROP INDEX shop_synonyms_en_idx;

-- A view names the rules table and not the index, so it works with the index and without it.
CREATE VIEW sofa_products AS SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms);
SELECT id FROM sofa_products ORDER BY id;
DROP INDEX shop_synonyms_idx;
SELECT id FROM sofa_products ORDER BY id;
DROP VIEW sofa_products;

-- Only `|||` and `&&&` expand.
SELECT id FROM products
WHERE description ### (SELECT pdb.expand('sofa', rule) FROM shop_synonyms);

-- A text column works too, and a value that is not a rule is an error.
CREATE TABLE text_rules (rule text);
INSERT INTO text_rules VALUES ('sofa, couch'), (NULL), ('# seats');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM text_rules) ORDER BY id;
INSERT INTO text_rules VALUES ('sofa => couch => divan');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM text_rules) ORDER BY id;

-- The aggregate takes one query text.
SELECT pdb.expand(texts.query, rule) FROM shop_synonyms, (VALUES ('a'), ('b')) AS texts(query);

DROP TABLE products, en_products, shop_synonyms, text_rules;

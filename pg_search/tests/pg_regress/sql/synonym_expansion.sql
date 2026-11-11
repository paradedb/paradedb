-- `pdb.with_synonyms` makes a match query expand with the rules of a synonym index.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 2;
DROP VIEW IF EXISTS sofa_products;
DROP TABLE IF EXISTS products, json_products, en_products, shop_synonyms, en_synonyms CASCADE;
DROP ROLE IF EXISTS synonym_expansion_reader;

CREATE TABLE shop_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO shop_synonyms VALUES
    ('sofa, couch, divan'),
    ('sneakers => running shoes'),
    ('tv, television');
CREATE INDEX shop_synonyms_idx ON shop_synonyms USING paradedb (rule);

CREATE TABLE products (id int PRIMARY KEY, description text);
INSERT INTO products VALUES
    (1, 'red couch'),
    (2, 'leather sofa'),
    (3, 'oak divan'),
    (4, 'running shoes for trails'),
    (5, 'shoes for running'),
    (6, 'blue sneakers'),
    (7, 'television stand'),
    (8, 'sofa and couch set');
CREATE INDEX products_idx ON products USING bm25 (id, description);

-- A plain match searches only its own terms.
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;

SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx') ORDER BY id;
SELECT id FROM products
WHERE description &&& pdb.with_synonyms('red sofa', 'shop_synonyms_idx') ORDER BY id;

-- A multi-word alternative is a phrase, and a directed rule drops the typed word.
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sneakers', 'shop_synonyms_idx') ORDER BY id;

-- A run of words that a rule lists is searched as a phrase, so `shoes for running` drops out.
INSERT INTO shop_synonyms VALUES ('running shoes, trainers');
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('running shoes', 'shop_synonyms_idx') ORDER BY id;
DELETE FROM shop_synonyms WHERE rule = 'running shoes, trainers';

-- A group scores as one term, so a row with two of its words scores a little above a row with
-- one.
SELECT id, round(pdb.score(id)::numeric, 4) AS score FROM products
WHERE description ||| pdb.with_synonyms('couch', 'shop_synonyms_idx') ORDER BY score DESC, id;

-- A fuzzy match keeps the typed word fuzzy and matches the alternatives exactly, so `sofas`
-- matches a fuzzy `sofa` and not a fuzzy `couch`. A directed rule drops the typed word. The cast
-- goes on the text or on the result.
INSERT INTO products VALUES (10, 'two sofas');
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx')::pdb.fuzzy(1) ORDER BY id;
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('couch'::pdb.fuzzy(1), 'shop_synonyms_idx') ORDER BY id;
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sneakers', 'shop_synonyms_idx')::pdb.fuzzy(1) ORDER BY id;
-- `&&&` keeps both fuzzy groups of one rule, and `sofas` matches only the first.
SELECT id FROM products
WHERE description &&& pdb.with_synonyms('sofa couch', 'shop_synonyms_idx')::pdb.fuzzy(1) ORDER BY id;
DELETE FROM products WHERE id = 10;

-- A boost keeps the expansion.
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx')::pdb.boost(2) ORDER BY id;

-- A parallel worker expands under the leader's snapshot.
SET debug_parallel_query = on;
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx') ORDER BY id;
RESET debug_parallel_query;

-- A generic plan expands the parameter at execution.
PREPARE sofa_query(text) AS SELECT id FROM products
WHERE description ||| pdb.with_synonyms($1, 'shop_synonyms_idx') ORDER BY id;
SET plan_cache_mode = force_generic_plan;
EXECUTE sofa_query('sofa');
RESET plan_cache_mode;
DEALLOCATE sofa_query;

-- The setting caps the alternatives, and the typed word stays.
SET paradedb.synonyms_max_alternatives = 2;
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('divan', 'shop_synonyms_idx') ORDER BY id;
RESET paradedb.synonyms_max_alternatives;

-- A rule change is live on the next query.
INSERT INTO products VALUES (9, 'velvet settee');
INSERT INTO shop_synonyms VALUES ('sofa, settee');
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('settee', 'shop_synonyms_idx') ORDER BY id;
DELETE FROM shop_synonyms WHERE rule = 'sofa, settee';
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('settee', 'shop_synonyms_idx') ORDER BY id;

-- The role that runs the query needs SELECT on the rules table, and must be exempt from row
-- security on it.
CREATE ROLE synonym_expansion_reader;
GRANT SELECT ON products TO synonym_expansion_reader;
SET ROLE synonym_expansion_reader;
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx') ORDER BY id;
RESET ROLE;
GRANT SELECT ON shop_synonyms TO synonym_expansion_reader;
SET ROLE synonym_expansion_reader;
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx') ORDER BY id;
RESET ROLE;
ALTER TABLE shop_synonyms ENABLE ROW LEVEL SECURITY;
SET ROLE synonym_expansion_reader;
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx') ORDER BY id;
RESET ROLE;
ALTER TABLE shop_synonyms DISABLE ROW LEVEL SECURITY;
REVOKE SELECT ON products, shop_synonyms FROM synonym_expansion_reader;
DROP ROLE synonym_expansion_reader;

-- A view keeps the synonym index and not its name, so a rename does not break the view.
CREATE VIEW sofa_products AS SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx');
ALTER INDEX shop_synonyms_idx RENAME TO shop_rules_idx;
SELECT id FROM sofa_products ORDER BY id;
ALTER INDEX shop_rules_idx RENAME TO shop_synonyms_idx;
DROP VIEW sofa_products;

-- A JSON field expands the same way, with a group and with a phrase.
CREATE TABLE json_products (id int PRIMARY KEY, attrs jsonb);
INSERT INTO json_products VALUES (1, '{"seat": "red couch"}'), (2, '{"shoes": "running shoes"}');
CREATE INDEX json_products_idx ON json_products USING bm25 (id, attrs);
SELECT id FROM json_products
WHERE attrs->>'seat' ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx') ORDER BY id;
SELECT id FROM json_products
WHERE attrs->>'shoes' ||| pdb.with_synonyms('sneakers', 'shop_synonyms_idx') ORDER BY id;
DROP TABLE json_products;

-- Keys are stored analyzed, so the synonym index must use the tokenizer of the field.
CREATE TABLE en_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO en_synonyms VALUES ('couch, sofa');
CREATE INDEX en_synonyms_idx ON en_synonyms
    USING paradedb ((rule::pdb.simple('stemmer=english')));
CREATE TABLE en_products (id int PRIMARY KEY, description text);
INSERT INTO en_products VALUES (1, 'two couches'), (2, 'a sofa'), (3, 'a divan');
CREATE INDEX en_products_idx ON en_products USING bm25 (
    id, (description::pdb.simple('stemmer=english'))
);
SELECT id FROM en_products
WHERE description ||| pdb.with_synonyms('sofas', 'en_synonyms_idx') ORDER BY id;
SELECT id FROM products
WHERE description ||| pdb.with_synonyms('sofa', 'en_synonyms_idx') ORDER BY id;

-- Only `|||` and `&&&` expand, and only a text value.
SELECT id FROM products WHERE description @@@ pdb.with_synonyms('sofa', 'shop_synonyms_idx');
SELECT pdb.with_synonyms(pdb.term('sofa'), 'shop_synonyms_idx');

-- The second argument must be a valid synonym index.
SELECT id FROM products WHERE description ||| pdb.with_synonyms('sofa', 'products_idx');
SELECT id FROM products WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms');
UPDATE pg_index SET indisvalid = false WHERE indexrelid = 'shop_synonyms_idx'::regclass;
SELECT id FROM products WHERE description ||| pdb.with_synonyms('sofa', 'shop_synonyms_idx');
UPDATE pg_index SET indisvalid = true WHERE indexrelid = 'shop_synonyms_idx'::regclass;

DROP TABLE products, en_products, shop_synonyms, en_synonyms;

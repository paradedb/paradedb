-- A match query expands with the synonym index that its index or its field names.

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS products, en_products, shop_synonyms, en_synonyms CASCADE;
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

-- Without the option, a query searches only its own terms.
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;

ALTER INDEX products_idx SET (synonyms = 'shop_synonyms_idx');
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;
SELECT id FROM products WHERE description &&& 'red sofa' ORDER BY id;

-- A multi-word alternative is a phrase, and a directed rule drops the typed word.
SELECT id FROM products WHERE description ||| 'sneakers' ORDER BY id;

-- A group scores as one term, so a row with two of its words scores a little above a row with
-- one.
SELECT id, round(pdb.score(id)::numeric, 4) AS score
FROM products WHERE description ||| 'couch' ORDER BY score DESC, id;

-- A fuzzy match keeps the typed word fuzzy and matches the alternatives exactly, so `sofas`
-- matches a fuzzy `sofa` and not a fuzzy `couch`. A directed rule drops the typed word.
INSERT INTO products VALUES (10, 'two sofas');
SELECT id FROM products WHERE description ||| 'sofa'::pdb.fuzzy(1) ORDER BY id;
SELECT id FROM products WHERE description ||| 'couch'::pdb.fuzzy(1) ORDER BY id;
SELECT id FROM products WHERE description ||| 'sneakers'::pdb.fuzzy(1) ORDER BY id;
DELETE FROM products WHERE id = 10;

-- A parallel worker expands under the leader's snapshot.
SET debug_parallel_query = on;
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;
RESET debug_parallel_query;

-- A tokenizer cast in the query, or the setting, turns expansion off.
SELECT id FROM products WHERE description ||| 'sofa'::pdb.simple ORDER BY id;
SET paradedb.enable_synonyms = off;
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;
RESET paradedb.enable_synonyms;

-- The setting caps the alternatives, and the typed word stays.
SET paradedb.synonyms_max_alternatives = 2;
SELECT id FROM products WHERE description ||| 'divan' ORDER BY id;
RESET paradedb.synonyms_max_alternatives;

-- A rule change is live on the next query.
INSERT INTO products VALUES (9, 'velvet settee');
INSERT INTO shop_synonyms VALUES ('sofa, settee');
SELECT id FROM products WHERE description ||| 'settee' ORDER BY id;
DELETE FROM shop_synonyms WHERE rule = 'sofa, settee';
SELECT id FROM products WHERE description ||| 'settee' ORDER BY id;

-- The owner of the index that names the rules attached them, so a query reads the rules with no
-- rights on the rules table, and row security on it does not apply.
CREATE ROLE synonym_expansion_reader;
GRANT SELECT ON products TO synonym_expansion_reader;
ALTER TABLE shop_synonyms ENABLE ROW LEVEL SECURITY;
SET ROLE synonym_expansion_reader;
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;
RESET ROLE;
ALTER TABLE shop_synonyms DISABLE ROW LEVEL SECURITY;
REVOKE SELECT ON products FROM synonym_expansion_reader;
DROP ROLE synonym_expansion_reader;

-- A bare name is looked up in the schema of the index that names it, and a name can carry its
-- schema.
CREATE SCHEMA shop;
CREATE TABLE shop.synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO shop.synonyms VALUES ('sofa, settee');
CREATE INDEX synonyms_idx ON shop.synonyms USING paradedb (rule);
CREATE TABLE shop.products (id int PRIMARY KEY, description text);
INSERT INTO shop.products VALUES (1, 'a settee'), (2, 'a sofa');
CREATE INDEX products_idx ON shop.products USING bm25 (id, description) WITH (synonyms = 'synonyms_idx');
SET search_path = public;
SELECT id FROM shop.products WHERE description ||| 'sofa' ORDER BY id;
ALTER INDEX shop.products_idx SET (synonyms = 'shop.synonyms_idx');
SELECT id FROM shop.products WHERE description ||| 'sofa' ORDER BY id;
RESET search_path;
DROP SCHEMA shop CASCADE;

-- The field option comes before the index option. Keys are stored analyzed, so the synonym
-- index must use the tokenizer of the field.
CREATE TABLE en_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO en_synonyms VALUES ('couch, sofa');
CREATE INDEX en_synonyms_idx ON en_synonyms
    USING paradedb ((rule::pdb.simple('stemmer=english')));
CREATE TABLE en_products (id int PRIMARY KEY, description text);
INSERT INTO en_products VALUES (1, 'two couches'), (2, 'a sofa'), (3, 'a divan');
CREATE INDEX en_products_idx ON en_products USING bm25 (
    id,
    (description::pdb.simple('stemmer=english', 'synonyms=en_synonyms_idx'))
) WITH (synonyms = 'shop_synonyms_idx');
SELECT id FROM en_products WHERE description ||| 'sofas' ORDER BY id;

-- The index option covers only the fields that share the tokenizer of the synonym index, and a
-- field option with another tokenizer fails.
ALTER INDEX products_idx SET (synonyms = 'en_synonyms_idx');
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;
CREATE TABLE mixed_products (id int PRIMARY KEY, description text);
CREATE INDEX mixed_products_idx ON mixed_products USING bm25 (
    id, (description::pdb.unicode_words('synonyms=en_synonyms_idx'))
);
SELECT id FROM mixed_products WHERE description ||| 'sofa';
DROP TABLE mixed_products;

-- The name is resolved on each query, so these fail at query time.
ALTER INDEX products_idx SET (synonyms = 'missing_idx');
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;
ALTER INDEX products_idx SET (synonyms = 'en_products_idx');
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;
ALTER INDEX products_idx SET (synonyms = 'shop_synonyms');
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;
ALTER INDEX products_idx RESET (synonyms);
SELECT id FROM products WHERE description ||| 'sofa' ORDER BY id;

DROP TABLE products, en_products, shop_synonyms, en_synonyms;

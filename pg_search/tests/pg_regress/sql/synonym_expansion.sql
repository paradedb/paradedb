-- A match query expands with the rules of a synonym index, which a `pdb.expand` subquery on the
-- rules table reads.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 2;
DROP TABLE IF EXISTS products, json_products, en_products, shop_synonyms, en_synonyms CASCADE;

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
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
SELECT id FROM products
WHERE description &&& (SELECT pdb.expand('red sofa', rule) FROM shop_synonyms) ORDER BY id;

-- EXPLAIN names the synonym index on its own line.
EXPLAIN (COSTS OFF)
SELECT id FROM products WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms);

-- A multi-word alternative is a phrase, and a directed rule drops the typed word.
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sneakers', rule) FROM shop_synonyms) ORDER BY id;

-- A run of words that a rule lists is searched as a phrase, so `shoes for running` drops out.
INSERT INTO shop_synonyms VALUES ('running shoes, trainers');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('running shoes', rule) FROM shop_synonyms) ORDER BY id;
DELETE FROM shop_synonyms WHERE rule = 'running shoes, trainers';

-- A word in two rules gets the alternatives of both. Rules do not chain, so `blue` does not
-- reach `oak` through `red`.
INSERT INTO shop_synonyms VALUES ('red, blue'), ('red, oak');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('red', rule) FROM shop_synonyms) ORDER BY id;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('blue', rule) FROM shop_synonyms) ORDER BY id;
DELETE FROM shop_synonyms WHERE rule IN ('red, blue', 'red, oak');

-- A fuzzy match keeps the typed word fuzzy and matches the alternatives exactly, so `sofas`
-- matches a fuzzy `sofa` and not a fuzzy `couch`. A directed rule drops the typed word.
INSERT INTO products VALUES (10, 'two sofas');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms)::pdb.fuzzy(1)
ORDER BY id;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('couch', rule) FROM shop_synonyms)::pdb.fuzzy(1)
ORDER BY id;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sneakers', rule) FROM shop_synonyms)::pdb.fuzzy(1)
ORDER BY id;
-- `&&&` keeps both fuzzy groups of one rule, and `sofas` matches only the first.
SELECT id FROM products
WHERE description &&& (SELECT pdb.expand('sofa couch', rule) FROM shop_synonyms)::pdb.fuzzy(1)
ORDER BY id;
DELETE FROM products WHERE id = 10;

-- A boost keeps the expansion.
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms)::pdb.boost(2)
ORDER BY id;

-- A parallel worker expands under the leader's snapshot.
SET debug_parallel_query = on;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
RESET debug_parallel_query;

-- The setting caps the alternatives, and the typed word stays.
SET paradedb.synonyms_max_alternatives = 2;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('divan', rule) FROM shop_synonyms) ORDER BY id;
RESET paradedb.synonyms_max_alternatives;

-- A rule change is live on the next query.
INSERT INTO products VALUES (9, 'velvet settee');
INSERT INTO shop_synonyms VALUES ('sofa, settee');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('settee', rule) FROM shop_synonyms) ORDER BY id;
DELETE FROM shop_synonyms WHERE rule = 'sofa, settee';
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('settee', rule) FROM shop_synonyms) ORDER BY id;

-- A backend keeps the rules it read, by segment, and checks the row of each rule on every
-- query, so an update, a reused ctid after VACUUM or a REINDEX never expands with an old rule.
INSERT INTO products VALUES (11, 'telly remote'), (12, 'monitor arm');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('tv', rule) FROM shop_synonyms) ORDER BY id;
UPDATE shop_synonyms SET rule = 'tv, telly' WHERE rule = 'tv, television';
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('tv', rule) FROM shop_synonyms) ORDER BY id;
VACUUM shop_synonyms;
INSERT INTO shop_synonyms VALUES ('tv, monitor');
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('tv', rule) FROM shop_synonyms) ORDER BY id;
REINDEX INDEX shop_synonyms_idx;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('tv', rule) FROM shop_synonyms) ORDER BY id;
DELETE FROM products WHERE id IN (11, 12);

-- A JSON field expands the same way, with a group and with a phrase.
CREATE TABLE json_products (id int PRIMARY KEY, attrs jsonb);
INSERT INTO json_products VALUES (1, '{"seat": "red couch"}'), (2, '{"shoes": "running shoes"}');
CREATE INDEX json_products_idx ON json_products USING bm25 (id, attrs);
SELECT id FROM json_products
WHERE attrs->>'seat' ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
SELECT id FROM json_products
WHERE attrs->>'shoes' ||| (SELECT pdb.expand('sneakers', rule) FROM shop_synonyms) ORDER BY id;
DROP TABLE json_products;

-- Keys are stored analyzed, so a query reads a synonym index that has the tokenizer of the field.
CREATE TABLE en_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO en_synonyms VALUES ('couch, sofa');
CREATE INDEX en_synonyms_idx ON en_synonyms
    USING paradedb ((rule::pdb.simple('stemmer=english')));
CREATE TABLE en_products (id int PRIMARY KEY, description text);
INSERT INTO en_products VALUES (1, 'two couches'), (2, 'a sofa'), (3, 'a divan');
CREATE INDEX en_products_idx ON en_products USING bm25 (
    id, (description::pdb.simple('stemmer=english'))
);
EXPLAIN (COSTS OFF)
SELECT id FROM en_products
WHERE description ||| (SELECT pdb.expand('sofas', rule) FROM en_synonyms);
SELECT id FROM en_products
WHERE description ||| (SELECT pdb.expand('sofas', rule) FROM en_synonyms) ORDER BY id;

-- A query never names the oid of an index, but a query value can hold one. The oid must be a
-- valid synonym index.
SELECT id FROM products
WHERE description ||| format('{"unclassified_string":{"string":"sofa","synonyms":%s}}',
    'products_idx'::regclass::oid)::pdb.query;
UPDATE pg_index SET indisvalid = false WHERE indexrelid = 'shop_synonyms_idx'::regclass;
SELECT id FROM products
WHERE description ||| format('{"unclassified_string":{"string":"sofa","synonyms":%s}}',
    'shop_synonyms_idx'::regclass::oid)::pdb.query;
UPDATE pg_index SET indisvalid = true WHERE indexrelid = 'shop_synonyms_idx'::regclass;

DROP TABLE products, en_products, shop_synonyms, en_synonyms;

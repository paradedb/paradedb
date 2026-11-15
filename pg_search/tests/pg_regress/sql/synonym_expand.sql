-- A `pdb.expand` subquery on a rules table makes a match query expand with the rules it reads.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 2;
DROP VIEW IF EXISTS sofa_products;
DROP TABLE IF EXISTS products, parted_products, en_products, shop_synonyms, text_rules CASCADE;
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

-- The planner runs a subquery that no index holds, for the row estimate of the match. A generic
-- plan has no text to run it with, so it keeps the estimate of an unknown value.
CREATE FUNCTION plan_rows(query text) RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE plan json;
BEGIN
    EXECUTE 'EXPLAIN (FORMAT JSON) ' || query INTO plan;
    RETURN plan->0->'Plan'->>'Plan Rows';
END $$;
SET paradedb.planner_warnings = 'off';
SELECT plan_rows($$SELECT id FROM products
    WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = 'main')$$);
PREPARE sofa_rows(text) AS SELECT id FROM products
WHERE description ||| (SELECT pdb.expand($1, rule) FROM shop_synonyms WHERE shop = 'main');
SET plan_cache_mode = force_generic_plan;
SELECT plan_rows($$EXECUTE sofa_rows('sofa')$$);
RESET plan_cache_mode;
DEALLOCATE sofa_rows;
RESET paradedb.planner_warnings;
DROP FUNCTION plan_rows;

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

-- A word reads the first `paradedb.synonyms_max_rules_per_key` rules that list it, so `sofa` does
-- not reach `settee`.
SET paradedb.synonyms_max_rules_per_key = 1;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
RESET paradedb.synonyms_max_rules_per_key;

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

-- An ORM can add `LIMIT 1` to a scalar subquery. The aggregate returns one row, so the index is
-- still read. `LIMIT 0` returns no row, so that subquery runs as written and matches nothing.
EXPLAIN (COSTS OFF)
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms LIMIT 1);
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms LIMIT 0);

-- A cast inside the subquery is more than the aggregate, so the subquery runs as written.
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule)::pdb.boost(2) FROM shop_synonyms)
ORDER BY id;

-- The index of a partitioned table has no storage. The query still reads the synonym index.
CREATE TABLE parted_products (id int PRIMARY KEY, description text) PARTITION BY RANGE (id);
CREATE TABLE parted_products_low PARTITION OF parted_products FOR VALUES FROM (0) TO (5);
CREATE TABLE parted_products_high PARTITION OF parted_products FOR VALUES FROM (5) TO (100);
INSERT INTO parted_products SELECT id, description FROM products;
CREATE INDEX parted_products_idx ON parted_products USING bm25 (id, description);
SELECT id FROM parted_products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
DROP TABLE parted_products;

-- The index holds the rows of the table itself. With a child table, `ONLY` reads those rows.
CREATE TABLE more_synonyms () INHERITS (shop_synonyms);
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM ONLY shop_synonyms) ORDER BY id;
DROP TABLE more_synonyms;

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
-- The cached plan is planned again when the synonym index goes away and when it comes back.
DROP INDEX shop_synonyms_idx;
EXECUTE sofa_query('sofa');
CREATE INDEX shop_synonyms_idx ON shop_synonyms USING paradedb (rule);
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
-- A custom plan has the value of a parameter in the filter and picks the partial index. A
-- generic plan has no value, so it can't.
PREPARE outlet_query(text) AS SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms WHERE shop = $1)
ORDER BY id;
SET plan_cache_mode = force_custom_plan;
EXECUTE outlet_query('outlet');
SET plan_cache_mode = force_generic_plan;
EXECUTE outlet_query('outlet');
RESET plan_cache_mode;
DEALLOCATE outlet_query;
DROP INDEX outlet_synonyms_idx;
DELETE FROM shop_synonyms WHERE shop = 'outlet';
CREATE INDEX shop_synonyms_idx ON shop_synonyms USING paradedb (rule);

-- A synonym index that is not valid is not read.
UPDATE pg_index SET indisvalid = false WHERE indexrelid = 'shop_synonyms_idx'::regclass;
SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
UPDATE pg_index SET indisvalid = true WHERE indexrelid = 'shop_synonyms_idx'::regclass;

-- A subquery with a filter that no index has reads the rules itself, and
-- `paradedb.planner_warnings` makes that an error.
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

-- A view reads the rules with the rights of its owner. The index is read with the rights of the
-- role that runs the query, so under row security for that role the subquery runs as written.
CREATE VIEW sofa_products AS SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms);
GRANT SELECT ON sofa_products TO synonym_expand_reader;
SET ROLE synonym_expand_reader;
SELECT id FROM sofa_products ORDER BY id;
RESET ROLE;
DROP VIEW sofa_products;
DELETE FROM shop_synonyms WHERE shop = 'outlet';
DROP POLICY outlet_rules ON shop_synonyms;
ALTER TABLE shop_synonyms DISABLE ROW LEVEL SECURITY;

-- A cached plan that reads the index is planned again for another role. A column grant is
-- enough for the subquery and not for the index, so that role reads the rules itself.
REVOKE SELECT ON shop_synonyms FROM synonym_expand_reader;
GRANT SELECT (rule) ON shop_synonyms TO synonym_expand_reader;
PREPARE role_query AS SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms) ORDER BY id;
EXECUTE role_query;
SET ROLE synonym_expand_reader;
EXECUTE role_query;
RESET ROLE;
DEALLOCATE role_query;
REVOKE SELECT ON products FROM synonym_expand_reader;
REVOKE SELECT (rule) ON shop_synonyms FROM synonym_expand_reader;

-- A view reads the rules with the rights of its owner. A run for the estimate would have the
-- rights of the role, which has none on the rules table, so the planner does not run it.
CREATE VIEW sofa_products AS SELECT id FROM products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM shop_synonyms);
GRANT SELECT ON sofa_products TO synonym_expand_reader;
SET ROLE synonym_expand_reader;
SELECT id FROM sofa_products ORDER BY id;
RESET ROLE;
DROP VIEW sofa_products;
DROP ROLE synonym_expand_reader;

-- A synonym index with another tokenizer than the field is not read, and the query still expands.
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

-- A window frame reads the aggregate after each row, so the final function keeps the state.
SELECT rule, (pdb.expand('sofa', rule) OVER (ORDER BY rule))::text::jsonb
    #> '{unclassified_string,synonyms}' AS rules
FROM shop_synonyms ORDER BY rule;

-- The aggregate takes one query text.
SELECT pdb.expand(texts.query, rule) FROM shop_synonyms, (VALUES ('a'), ('b')) AS texts(query);

DROP TABLE products, en_products, shop_synonyms, text_rules;

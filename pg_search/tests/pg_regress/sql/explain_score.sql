-- `pdb.explain_score` gives the explanation of the score of a row, and a synonym group explains
-- as one term with the words of the group that the row holds.

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS explain_products, explain_orders, explain_synonyms;

CREATE TABLE explain_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO explain_synonyms VALUES
    ('sofa, couch, divan'),
    ('trainers, running shoes');
CREATE INDEX explain_synonyms_idx ON explain_synonyms USING paradedb (rule);

CREATE TABLE explain_products (id int PRIMARY KEY, description text);
INSERT INTO explain_products VALUES
    (1, 'red couch'),
    (2, 'red leather sofa and a couch'),
    (3, 'oak divan'),
    (4, 'blue running shoes'),
    (5, 'red chair'),
    (6, 'white trainers');
CREATE INDEX explain_products_idx ON explain_products USING bm25 (id, description);

-- A term explains with its idf, its frequency in the row and the length of the field. The query
-- does not select `pdb.score`, and the scan still computes scores.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, jsonb_pretty(pdb.explain_score(id)) FROM explain_products
WHERE description ||| 'oak' ORDER BY id;
SELECT id, jsonb_pretty(pdb.explain_score(id)) FROM explain_products
WHERE description ||| 'oak' ORDER BY id;

-- A synonym group is one term. Its frequency is the sum over the words of the group that the
-- row holds, and row 2 holds two of them.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id,
       pdb.explain_score(id)->>'description' AS description,
       jsonb_pretty(pdb.explain_score(id) #> '{details,2,details,0}') AS frequency
FROM explain_products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM explain_synonyms) ORDER BY id;
SELECT id,
       pdb.explain_score(id)->>'description' AS description,
       jsonb_pretty(pdb.explain_score(id) #> '{details,2,details,0}') AS frequency
FROM explain_products
WHERE description ||| (SELECT pdb.expand('sofa', rule) FROM explain_synonyms) ORDER BY id;

-- A group with a multi-word alternative takes the best of its alternatives, and the explanation
-- has the one that the row holds.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id,
       pdb.explain_score(id)->>'description' AS description,
       jsonb_path_query_array(pdb.explain_score(id), '$.details[*].description') AS alternatives
FROM explain_products
WHERE description ||| (SELECT pdb.expand('trainers', rule) FROM explain_synonyms) ORDER BY id;
SELECT id,
       pdb.explain_score(id)->>'description' AS description,
       jsonb_path_query_array(pdb.explain_score(id), '$.details[*].description') AS alternatives
FROM explain_products
WHERE description ||| (SELECT pdb.expand('trainers', rule) FROM explain_synonyms) ORDER BY id;

-- The explanation of a Top K row has the score of that row.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id), (pdb.explain_score(id)->>'value')::real = pdb.score(id) AS same
FROM explain_products
WHERE description ||| (SELECT pdb.expand('red sofa', rule) FROM explain_synonyms)
ORDER BY pdb.score(id) DESC, id LIMIT 3;
SELECT id, pdb.score(id), (pdb.explain_score(id)->>'value')::real = pdb.score(id) AS same
FROM explain_products
WHERE description ||| (SELECT pdb.expand('red sofa', rule) FROM explain_synonyms)
ORDER BY pdb.score(id) DESC, id LIMIT 3;

-- A row that matches one side of an `OR` explains with that side, also when the other side
-- first matches a later row.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id), (pdb.explain_score(id)->>'value')::real = pdb.score(id) AS same
FROM explain_products WHERE description ||| 'red' OR id > 4 ORDER BY id;
SELECT id, pdb.score(id), (pdb.explain_score(id)->>'value')::real = pdb.score(id) AS same
FROM explain_products WHERE description ||| 'red' OR id > 4 ORDER BY id;

-- The Join Scan does not explain a score, so the Base Scan of the table does.
CREATE TABLE explain_orders (id int PRIMARY KEY, product_id int, note text);
INSERT INTO explain_orders VALUES (1, 1, 'fast'), (2, 2, 'slow'), (3, 5, 'fast');
CREATE INDEX explain_orders_idx ON explain_orders USING bm25 (id, product_id, note);
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT o.id, p.id, (pdb.explain_score(p.id)->>'value')::real = pdb.score(p.id) AS same
FROM explain_orders o JOIN explain_products p ON p.id = o.product_id
WHERE p.description ||| 'red' ORDER BY pdb.score(p.id) DESC, o.id LIMIT 2;
SELECT o.id, p.id, (pdb.explain_score(p.id)->>'value')::real = pdb.score(p.id) AS same
FROM explain_orders o JOIN explain_products p ON p.id = o.product_id
WHERE p.description ||| 'red' ORDER BY pdb.score(p.id) DESC, o.id LIMIT 2;

-- The scan gives the explanation with its output, so a `WHERE` clause can't read it.
SELECT id FROM explain_products
WHERE description ||| 'red' AND (pdb.explain_score(id)->>'value')::real > 0.5;

-- A query with no search operator has no score to explain.
SELECT id, pdb.explain_score(id) FROM explain_products WHERE id = 1;

DROP TABLE explain_products, explain_orders, explain_synonyms;

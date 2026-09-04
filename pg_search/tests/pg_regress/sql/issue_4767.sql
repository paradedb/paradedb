-- Regression test for #4767: pdb.score / pdb.snippet without a ParadeDB
-- operator in WHERE used to raise the generic "Unsupported query shape"
-- message. They should name the operator requirement instead.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS issue_4767_products CASCADE;
CREATE TABLE issue_4767_products (
    id serial PRIMARY KEY,
    description text
);

INSERT INTO issue_4767_products (description) VALUES
    ('running shoes'),
    ('wireless keyboard');

CREATE INDEX issue_4767_products_idx
    ON issue_4767_products
    USING paradedb (id, description)
    WITH (key_field = 'id');

\echo '--- pdb.score without a WHERE clause ---'
SELECT p.id, pdb.score(p.id) AS score
FROM issue_4767_products AS p;

\echo '--- pdb.snippet without a WHERE clause ---'
SELECT pdb.snippet(p.description, '<div>', '</div>', 235) AS snippet
FROM issue_4767_products AS p;

\echo '--- pdb.snippets without a WHERE clause ---'
SELECT pdb.snippets(p.description) AS snippets
FROM issue_4767_products AS p;

\echo '--- pdb.snippet_positions without a WHERE clause ---'
SELECT pdb.snippet_positions(p.description) AS positions
FROM issue_4767_products AS p;

\echo '--- paradedb.score without a WHERE clause ---'
SELECT paradedb.score(p.id) AS score
FROM issue_4767_products AS p;

\echo '--- paradedb.snippet without a WHERE clause ---'
SELECT paradedb.snippet(p.description) AS snippet
FROM issue_4767_products AS p;

\echo '--- paradedb.snippets without a WHERE clause ---'
SELECT paradedb.snippets(p.description) AS snippets
FROM issue_4767_products AS p;

\echo '--- paradedb.snippet_positions without a WHERE clause ---'
SELECT paradedb.snippet_positions(p.description) AS positions
FROM issue_4767_products AS p;

\echo '--- pdb.score with a non-search WHERE clause ---'
SELECT p.id, pdb.score(p.id) AS score
FROM issue_4767_products AS p
WHERE p.id = 1;

\echo '--- pdb.score with a ParadeDB operator still works ---'
SELECT p.id, pdb.score(p.id) AS score
FROM issue_4767_products AS p
WHERE p.description ||| 'shoes'
ORDER BY p.id;

-- Join-without-LIMIT + pdb.score is covered by join_tests. A self-join
-- of that shape trips a planner Assert (apply_tlist_labeling), so it is
-- not reproduced here.
\echo '--- pdb.score with an operator when custom scan is off ---'
SET paradedb.enable_custom_scan = off;
SELECT p.id, pdb.score(p.id) AS score
FROM issue_4767_products AS p
WHERE p.description ||| 'shoes';
SET paradedb.enable_custom_scan = on;

DROP TABLE issue_4767_products CASCADE;

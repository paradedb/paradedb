\i common/common_setup.sql
\set VERBOSITY default

CREATE TABLE projection_search_predicate (
    id INTEGER PRIMARY KEY,
    body TEXT,
    category TEXT
);

INSERT INTO projection_search_predicate VALUES
    (1, 'mechanical keyboard', 'electronics'),
    (2, 'wireless mouse', 'electronics'),
    (3, 'keyboard stand', 'accessories');

CREATE INDEX projection_search_predicate_idx
ON projection_search_predicate USING paradedb (id, body);

-- PostgreSQL drops the redundant branch, and the search predicate with it. The scan runs on
-- what remains and the projections see no search terms, as with `pdb.all()`.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, pdb.score(id)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

SELECT id, pdb.score(id)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body @@@ 'keyboard')
ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.snippet(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, pdb.snippet(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

SELECT id, pdb.snippet(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body @@@ 'keyboard')
ORDER BY id;

SELECT id, pdb.snippets(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

SELECT id, pdb.snippet_positions(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

SELECT id, paradedb.score(id)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

SELECT id, paradedb.snippet(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

SELECT id, paradedb.snippets(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

SELECT id, paradedb.snippet_positions(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

-- Without a score or snippet, the scan is one candidate among PostgreSQL's.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

-- A sibling subquery over the same table, under the same default alias, is not affected.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT (SELECT count(*) FROM projection_search_predicate WHERE body === 'keyboard') AS searched,
       (SELECT count(*) FROM projection_search_predicate WHERE category = 'electronics') AS plain;

-- The operator is written against one side of a join.
CREATE TABLE projection_search_predicate_labels (
    id INTEGER PRIMARY KEY,
    label TEXT
);
INSERT INTO projection_search_predicate_labels VALUES (1, 'x'), (2, 'y'), (3, 'z');

-- The score is computed inside a sublink of the SELECT list.
SELECT l.id,
       (SELECT pdb.score(p.id)
        FROM projection_search_predicate p
        WHERE p.id = l.id
          AND (p.category = 'electronics' OR (p.category = 'electronics' AND p.body === 'keyboard'))
       ) AS score
FROM projection_search_predicate_labels l
ORDER BY l.id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, l.label, pdb.score(p.id)
FROM projection_search_predicate p
JOIN projection_search_predicate_labels l ON p.id = l.id
WHERE p.category = 'electronics' OR (p.category = 'electronics' AND p.body === 'keyboard')
ORDER BY p.id;
SELECT p.id, l.label, pdb.score(p.id)
FROM projection_search_predicate p
JOIN projection_search_predicate_labels l ON p.id = l.id
WHERE p.category = 'electronics' OR (p.category = 'electronics' AND p.body === 'keyboard')
ORDER BY p.id;

-- The operator is written against a partitioned table; the partitions are scanned.
CREATE TABLE projection_search_predicate_parts (
    id INTEGER,
    body TEXT,
    category TEXT,
    yr INTEGER
) PARTITION BY RANGE (yr);
CREATE TABLE projection_search_predicate_parts_2020
    PARTITION OF projection_search_predicate_parts FOR VALUES FROM (2020) TO (2021);
CREATE TABLE projection_search_predicate_parts_2021
    PARTITION OF projection_search_predicate_parts FOR VALUES FROM (2021) TO (2022);
INSERT INTO projection_search_predicate_parts VALUES
    (1, 'mechanical keyboard', 'electronics', 2020),
    (2, 'wireless mouse', 'electronics', 2021),
    (3, 'keyboard stand', 'accessories', 2021);
CREATE INDEX projection_search_predicate_parts_idx
ON projection_search_predicate_parts USING paradedb (id, body);

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id)
FROM projection_search_predicate_parts
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, pdb.score(id)
FROM projection_search_predicate_parts
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

-- JoinScan and AggregateScan take the query on the same grounds.
CREATE TABLE projection_search_predicate_owners (
    id INTEGER PRIMARY KEY,
    product_id INTEGER,
    name TEXT
);
INSERT INTO projection_search_predicate_owners VALUES (1, 1, 'ann'), (2, 2, 'bob'), (3, 3, 'cid');
CREATE INDEX projection_search_predicate_owners_idx
ON projection_search_predicate_owners USING paradedb (id, product_id, name);

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT p.id, o.name, pdb.score(p.id)
FROM projection_search_predicate p
JOIN projection_search_predicate_owners o ON o.product_id = p.id
WHERE p.category = 'electronics' OR (p.category = 'electronics' AND p.body === 'keyboard')
ORDER BY p.id
LIMIT 10;
SELECT p.id, o.name, pdb.score(p.id)
FROM projection_search_predicate p
JOIN projection_search_predicate_owners o ON o.product_id = p.id
WHERE p.category = 'electronics' OR (p.category = 'electronics' AND p.body === 'keyboard')
ORDER BY p.id
LIMIT 10;

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT count(*)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard');
SELECT count(*)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard');

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT o.product_id, count(*)
FROM projection_search_predicate p
JOIN projection_search_predicate_owners o ON o.product_id = p.id
WHERE p.category = 'electronics' OR (p.category = 'electronics' AND p.body === 'keyboard')
GROUP BY o.product_id;
SELECT o.product_id, count(*)
FROM projection_search_predicate p
JOIN projection_search_predicate_owners o ON o.product_id = p.id
WHERE p.category = 'electronics' OR (p.category = 'electronics' AND p.body === 'keyboard')
GROUP BY o.product_id
ORDER BY o.product_id;

-- No search predicate was written, so none was simplified away.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id) FROM projection_search_predicate ORDER BY id;
SELECT id, pdb.score(id) FROM projection_search_predicate ORDER BY id;
\echo :SQLSTATE

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.snippet(body) FROM projection_search_predicate ORDER BY id;
SELECT id, pdb.snippet(body) FROM projection_search_predicate ORDER BY id;

-- The whole WHERE clause folds away, so the scan covers every row, as with `pdb.all()`.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id)
FROM projection_search_predicate
WHERE body === 'keyboard' OR TRUE
ORDER BY id;
SELECT id, pdb.score(id)
FROM projection_search_predicate
WHERE body === 'keyboard' OR TRUE
ORDER BY id;

PREPARE redundant_score(text, text) AS
SELECT id, pdb.score(id)
FROM projection_search_predicate
WHERE category = $1 OR (category = $1 AND body === $2)
ORDER BY id;

PREPARE redundant_snippet(text, text) AS
SELECT id, pdb.snippet(body)
FROM projection_search_predicate
WHERE category = $1 OR (category = $1 AND body @@@ $2)
ORDER BY id;

SET plan_cache_mode = force_custom_plan;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
EXECUTE redundant_score('electronics', 'keyboard');
EXECUTE redundant_score('electronics', 'keyboard');
EXECUTE redundant_snippet('electronics', 'keyboard');

SET plan_cache_mode = force_generic_plan;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
EXECUTE redundant_score('electronics', 'keyboard');
EXECUTE redundant_score('electronics', 'keyboard');
EXECUTE redundant_snippet('electronics', 'keyboard');
EXECUTE redundant_score('accessories', 'mouse');
EXECUTE redundant_snippet('accessories', 'mouse');

RESET plan_cache_mode;
DEALLOCATE redundant_score;
DEALLOCATE redundant_snippet;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id) > 0 AS has_score,
       pdb.snippet(body), pdb.snippets(body), pdb.snippet_positions(body)
FROM projection_search_predicate
WHERE body === 'keyboard'
ORDER BY id;
SELECT id, pdb.score(id) > 0 AS has_score,
       pdb.snippet(body), pdb.snippets(body), pdb.snippet_positions(body)
FROM projection_search_predicate
WHERE body === 'keyboard'
ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, paradedb.score(id) > 0 AS has_score,
       paradedb.snippet(body), paradedb.snippets(body), paradedb.snippet_positions(body)
FROM projection_search_predicate
WHERE body @@@ 'keyboard'
ORDER BY id;
SELECT id, paradedb.score(id) > 0 AS has_score,
       paradedb.snippet(body), paradedb.snippets(body), paradedb.snippet_positions(body)
FROM projection_search_predicate
WHERE body @@@ 'keyboard'
ORDER BY id;

DROP TABLE projection_search_predicate_owners;
DROP TABLE projection_search_predicate_parts;
DROP TABLE projection_search_predicate_labels;
DROP TABLE projection_search_predicate;

\i common/common_cleanup.sql

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

-- The search term cannot affect which rows satisfy the redundant branch.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, pdb.score(id)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
\echo :SQLSTATE

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body @@@ 'keyboard')
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

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.snippet(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body @@@ 'keyboard')
ORDER BY id;
SELECT id, pdb.snippet(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body @@@ 'keyboard')
ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.snippets(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, pdb.snippets(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.snippet_positions(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, pdb.snippet_positions(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, paradedb.score(id)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, paradedb.score(id)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, paradedb.snippet(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, paradedb.snippet(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, paradedb.snippets(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, paradedb.snippets(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, paradedb.snippet_positions(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;
SELECT id, paradedb.snippet_positions(body)
FROM projection_search_predicate
WHERE category = 'electronics' OR (category = 'electronics' AND body === 'keyboard')
ORDER BY id;

-- A missing search predicate does not imply that optimization removed one.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.score(id) FROM projection_search_predicate ORDER BY id;
SELECT id, pdb.score(id) FROM projection_search_predicate ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, pdb.snippet(body) FROM projection_search_predicate ORDER BY id;
SELECT id, pdb.snippet(body) FROM projection_search_predicate ORDER BY id;

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
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
EXECUTE redundant_snippet('electronics', 'keyboard');
EXECUTE redundant_snippet('electronics', 'keyboard');

SET plan_cache_mode = force_generic_plan;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
EXECUTE redundant_score('electronics', 'keyboard');
EXECUTE redundant_score('electronics', 'keyboard');
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
EXECUTE redundant_snippet('electronics', 'keyboard');
EXECUTE redundant_snippet('electronics', 'keyboard');
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
EXECUTE redundant_score('accessories', 'mouse');
EXECUTE redundant_score('accessories', 'mouse');
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
EXECUTE redundant_snippet('accessories', 'mouse');
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

DROP TABLE projection_search_predicate;

\i common/common_cleanup.sql

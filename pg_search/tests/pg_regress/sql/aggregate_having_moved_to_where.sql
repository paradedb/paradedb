-- PostgreSQL moves a HAVING condition that has no aggregate to WHERE. It does
-- the same with a condition of an outer query on a grouped subquery. The
-- Aggregate Scan must take such a query: no HAVING clause is left.

\i common/common_setup.sql

CREATE TABLE having_items (
    id SERIAL PRIMARY KEY,
    account_id BIGINT,
    kind TEXT
);

INSERT INTO having_items (account_id, kind)
SELECT (g % 3) + 1, (ARRAY['a', 'b', 'c', 'd'])[(g % 4) + 1]
FROM generate_series(1, 120) g;

CREATE INDEX having_items_idx ON having_items
USING paradedb (id, account_id, (kind::pdb.literal))
WITH (key_field = 'id');

SET paradedb.enable_aggregate_custom_scan TO on;

\echo 'Test 1: HAVING on a GROUP BY key'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, kind, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id, kind
HAVING account_id > 1
ORDER BY account_id, kind;

SELECT account_id, kind, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id, kind
HAVING account_id > 1
ORDER BY account_id, kind;

\echo 'Test 2: a condition of the outer query on a grouped subquery'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT *
FROM (
    SELECT account_id, kind, COUNT(*) AS n
    FROM having_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id, kind
) AS grouped
WHERE account_id > 1
ORDER BY account_id, kind;

SELECT *
FROM (
    SELECT account_id, kind, COUNT(*) AS n
    FROM having_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id, kind
) AS grouped
WHERE account_id > 1
ORDER BY account_id, kind;

\echo 'Test 3: the same through a CTE'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
WITH grouped AS (
    SELECT account_id, kind, COUNT(*) AS n
    FROM having_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id, kind
)
SELECT * FROM grouped WHERE kind >= 'c' ORDER BY account_id, kind;

WITH grouped AS (
    SELECT account_id, kind, COUNT(*) AS n
    FROM having_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id, kind
)
SELECT * FROM grouped WHERE kind >= 'c' ORDER BY account_id, kind;

\echo 'Test 4: a HAVING condition on an aggregate stays in HAVING -> declined'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT account_id, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING COUNT(*) > 10
ORDER BY account_id;

SELECT account_id, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING COUNT(*) > 10
ORDER BY account_id;

\echo 'Test 5: a condition of the outer query on the aggregate stays in HAVING -> declined'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT *
FROM (
    SELECT account_id, COUNT(*) AS n
    FROM having_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id
) AS grouped
WHERE n > 10
ORDER BY account_id;

SELECT *
FROM (
    SELECT account_id, COUNT(*) AS n
    FROM having_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id
) AS grouped
WHERE n > 10
ORDER BY account_id;

\echo 'Test 6: a constant true HAVING on an aggregate without GROUP BY'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM having_items WHERE id @@@ paradedb.all() HAVING 1 = 1;

SELECT COUNT(*) FROM having_items WHERE id @@@ paradedb.all() HAVING 1 = 1;

\echo 'Test 7: a constant false HAVING on an aggregate without GROUP BY stays in HAVING -> declined'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM having_items WHERE id @@@ paradedb.all() HAVING 1 = 2;

SELECT COUNT(*) FROM having_items WHERE id @@@ paradedb.all() HAVING 1 = 2;

\echo 'Test 8: a constant false HAVING with GROUP BY moves to WHERE'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT account_id, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING 1 = 2;

SELECT account_id, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING 1 = 2;

\echo 'Test 9: a volatile HAVING condition stays in HAVING -> declined'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT account_id, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING account_id > random() * 0
ORDER BY account_id;

SELECT account_id, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING account_id > random() * 0
ORDER BY account_id;

\echo 'Test 10: a moved condition that the index cannot answer is checked on the heap'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT account_id, kind, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id, kind
HAVING length(kind) = 1 AND kind <> 'b'
ORDER BY account_id, kind;

SELECT account_id, kind, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id, kind
HAVING length(kind) = 1 AND kind <> 'b'
ORDER BY account_id, kind;

\echo 'Test 11: a moved condition with a parameter in a generic plan'
PREPARE having_param(BIGINT) AS
SELECT account_id, kind, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id, kind
HAVING account_id > $1
ORDER BY account_id, kind;

SET plan_cache_mode TO force_generic_plan;
EXPLAIN (COSTS OFF, TIMING OFF) EXECUTE having_param(1);
EXECUTE having_param(1);
EXECUTE having_param(2);
RESET plan_cache_mode;
DEALLOCATE having_param;

\echo 'Test 12: pdb.agg() with a moved condition'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT account_id, pdb.agg('{"terms": {"field": "kind"}}'::jsonb)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING account_id > 1
ORDER BY account_id;

SELECT account_id, pdb.agg('{"terms": {"field": "kind"}}'::jsonb)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING account_id > 1
ORDER BY account_id;

\echo 'Test 13: pdb.agg() with a condition on an aggregate -> error'
SELECT account_id, pdb.agg('{"terms": {"field": "kind"}}'::jsonb)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING COUNT(*) > 10
ORDER BY account_id;

\echo 'Same results from PostgreSQL'
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT account_id, kind, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id, kind
HAVING account_id > 1
ORDER BY account_id, kind;

SELECT *
FROM (
    SELECT account_id, kind, COUNT(*) AS n
    FROM having_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id, kind
) AS grouped
WHERE account_id > 1
ORDER BY account_id, kind;

WITH grouped AS (
    SELECT account_id, kind, COUNT(*) AS n
    FROM having_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id, kind
)
SELECT * FROM grouped WHERE kind >= 'c' ORDER BY account_id, kind;

SELECT COUNT(*) FROM having_items WHERE id @@@ paradedb.all() HAVING 1 = 1;

SELECT account_id, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
HAVING 1 = 2;

SELECT account_id, kind, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id, kind
HAVING length(kind) = 1 AND kind <> 'b'
ORDER BY account_id, kind;

SELECT account_id, kind, COUNT(*)
FROM having_items
WHERE id @@@ paradedb.all()
GROUP BY account_id, kind
HAVING account_id > 2
ORDER BY account_id, kind;

RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE having_items;

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
USING paradedb (id, account_id, (kind::pdb.literal));

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

RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE having_items;

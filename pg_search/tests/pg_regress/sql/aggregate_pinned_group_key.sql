-- A GROUP BY key that the WHERE clause pins to one value (`key = constant`).
-- PostgreSQL does not group on such a key, and reads it from one row of the
-- group. AggregateScan must do the same.

\i common/common_setup.sql

CREATE COLLATION IF NOT EXISTS pinned_key_case_insensitive (
    provider = icu,
    locale = 'und-u-ks-level2',
    deterministic = false
);

CREATE TABLE pinned_key_items (
    id SERIAL PRIMARY KEY,
    account_id BIGINT,
    region SMALLINT,
    kind TEXT,
    code VARCHAR(10),
    label TEXT COLLATE pinned_key_case_insensitive,
    price FLOAT8,
    amount NUMERIC(10, 2),
    created DATE,
    updated TIMESTAMP,
    metadata JSONB
);

INSERT INTO pinned_key_items (account_id, region, kind, code, label, price, amount, created, updated, metadata)
SELECT
    (g % 3) + 1,
    (g % 3) + 1,
    (ARRAY['a', 'b', 'c', 'd'])[(g % 4) + 1],
    'c' || (g % 2),
    'label',
    g % 5,
    (g % 5) + 0.5,
    DATE '2024-01-01' + (g % 3),
    TIMESTAMP '2024-01-01 00:00:00' + (g % 3) * INTERVAL '1 day',
    jsonb_build_object('color', (ARRAY['red', 'blue'])[(g % 2) + 1])
FROM generate_series(1, 120) g;

CREATE INDEX pinned_key_items_idx ON pinned_key_items
USING paradedb (
    id, account_id, region, (kind::pdb.literal), (code::pdb.literal),
    (label::pdb.literal), price, amount, created, updated, metadata
);

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- SECTION 1: Every key is pinned
-- =====================================================================

\echo 'Test 1.1: the only key is pinned'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 1.2: no row matches -> no group'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 1.3: the key is not in the SELECT list'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 1.4: no aggregate'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id
FROM pinned_key_items
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 1.5: two pinned keys'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT code, account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 2 AND code = 'c1' AND id @@@ paradedb.all()
GROUP BY account_id, code;

SELECT code, account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 2 AND code = 'c1' AND id @@@ paradedb.all()
GROUP BY account_id, code;

\echo 'Test 1.6: FILTER aggregate -> a row if a row matches the WHERE clause'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, COUNT(*) FILTER (WHERE kind = 'none')
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*) FILTER (WHERE kind = 'none')
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*) FILTER (WHERE kind = 'none')
FROM pinned_key_items
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 1.7: a node above the scan reads the aggregate'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT account_id, COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT DISTINCT account_id, COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, COUNT(*), SUM(COUNT(*)) OVER ()
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*), SUM(COUNT(*)) OVER ()
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, generate_series(1, 2) AS n, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 1.8: aggregate with its own ORDER BY'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, COUNT(id ORDER BY kind)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(id ORDER BY kind)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

-- =====================================================================
-- SECTION 2: A pinned key next to a key that is not pinned
-- =====================================================================

\echo 'Test 2.1: pinned key and free key'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*) AS count_all, account_id, kind
FROM pinned_key_items
WHERE region = 1 AND account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT COUNT(*) AS count_all, account_id, kind
FROM pinned_key_items
WHERE region = 1 AND account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

\echo 'Test 2.2: no row matches'
SELECT COUNT(*) AS count_all, account_id, kind
FROM pinned_key_items
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

\echo 'Test 2.3: ORDER BY and LIMIT on the pinned key'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, kind, SUM(price)
FROM pinned_key_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY account_id, kind
LIMIT 3;

SELECT account_id, kind, SUM(price)
FROM pinned_key_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY account_id, kind
LIMIT 3;

\echo 'Test 2.4: aggregate in an expression'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, kind, COALESCE(SUM(price), 0) + 1 AS total
FROM pinned_key_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT account_id, kind, COALESCE(SUM(price), 0) + 1 AS total
FROM pinned_key_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

\echo 'Test 2.5: pdb.agg()'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, kind, pdb.agg('{"avg": {"field": "price"}}'::jsonb)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT account_id, kind, pdb.agg('{"avg": {"field": "price"}}'::jsonb)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT account_id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 2.6: pinned column that is not a GROUP BY key'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 3;

SELECT id, account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 3;

\echo 'Test 2.7: aggregate with its own ORDER BY'
SELECT account_id, code, COUNT(id ORDER BY kind), SUM(price ORDER BY kind)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, code
ORDER BY code;

-- =====================================================================
-- SECTION 3: The type of the constant and of the key
-- =====================================================================

\echo 'Test 3.1: constant of a different type than the key'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT region, pg_typeof(region), count
FROM (
    SELECT region, COUNT(*)
    FROM pinned_key_items
    WHERE region = 2::bigint AND id @@@ paradedb.all()
    GROUP BY region
) AS grouped;

SELECT region, pg_typeof(region), count
FROM (
    SELECT region, COUNT(*)
    FROM pinned_key_items
    WHERE region = 2::bigint AND id @@@ paradedb.all()
    GROUP BY region
) AS grouped;

\echo 'Test 3.2: constant that the key cannot hold -> no row, and no cast error'
SELECT region, COUNT(*)
FROM pinned_key_items
WHERE region = 100000 AND id @@@ paradedb.all()
GROUP BY region;

SELECT region, kind, COUNT(*)
FROM pinned_key_items
WHERE region = 100000 AND id @@@ paradedb.all()
GROUP BY region, kind;

\echo 'Test 3.3: varchar, date and JSON expression keys'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT code, kind, COUNT(*)
FROM pinned_key_items
WHERE code = 'c1' AND id @@@ paradedb.all()
GROUP BY code, kind
ORDER BY kind;

SELECT code, kind, COUNT(*)
FROM pinned_key_items
WHERE code = 'c1' AND id @@@ paradedb.all()
GROUP BY code, kind
ORDER BY kind;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT created, COUNT(*)
FROM pinned_key_items
WHERE created = '2024-01-02' AND id @@@ paradedb.all()
GROUP BY created;

SELECT created, COUNT(*)
FROM pinned_key_items
WHERE created = '2024-01-02' AND id @@@ paradedb.all()
GROUP BY created;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT metadata->>'color' AS color, kind, COUNT(*)
FROM pinned_key_items
WHERE metadata->>'color' = 'red' AND id @@@ paradedb.all()
GROUP BY metadata->>'color', kind
ORDER BY kind;

SELECT metadata->>'color' AS color, kind, COUNT(*)
FROM pinned_key_items
WHERE metadata->>'color' = 'red' AND id @@@ paradedb.all()
GROUP BY metadata->>'color', kind
ORDER BY kind;

\echo 'Test 3.4: expression key that is not in the index -> declined'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT upper(kind) AS upper_kind, COUNT(*)
FROM pinned_key_items
WHERE upper(kind) = 'A' AND id @@@ paradedb.all()
GROUP BY upper(kind);

SELECT upper(kind) AS upper_kind, COUNT(*)
FROM pinned_key_items
WHERE upper(kind) = 'A' AND id @@@ paradedb.all()
GROUP BY upper(kind);

-- =====================================================================
-- SECTION 4: The value is not known at plan time
-- =====================================================================

\echo 'Test 4.1: parameter of a generic plan'
SET plan_cache_mode = force_generic_plan;

PREPARE pinned_key_all(bigint) AS
SELECT account_id, COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = $1 AND id @@@ paradedb.all()
GROUP BY account_id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE pinned_key_all(1);
EXECUTE pinned_key_all(1);
EXECUTE pinned_key_all(2);
EXECUTE pinned_key_all(99);
EXECUTE pinned_key_all(NULL);

PREPARE pinned_key_mixed(bigint) AS
SELECT account_id, kind, COUNT(*)
FROM pinned_key_items
WHERE account_id = $1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

EXECUTE pinned_key_mixed(3);
EXECUTE pinned_key_mixed(1);
EXECUTE pinned_key_mixed(99);
EXECUTE pinned_key_mixed(2);

-- A value longer than the `varchar(10)` key matches no row.
PREPARE pinned_key_code(text) AS
SELECT code, kind, COUNT(*)
FROM pinned_key_items
WHERE code = $1 AND id @@@ paradedb.all()
GROUP BY code, kind
ORDER BY kind;

EXECUTE pinned_key_code('c1');
EXECUTE pinned_key_code('a value that is longer than ten characters');

DEALLOCATE pinned_key_all;
DEALLOCATE pinned_key_mixed;
DEALLOCATE pinned_key_code;
RESET plan_cache_mode;

\echo 'Test 4.2: subquery result'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = (SELECT 3) AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = (SELECT 3) AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 4.3: value of an outer query, one scan for each value'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT outer_values.account_id AS outer_account_id, grouped.*
FROM (VALUES (1::bigint), (2), (99)) AS outer_values (account_id),
LATERAL (
    SELECT account_id, COUNT(*)
    FROM pinned_key_items
    WHERE account_id = outer_values.account_id AND id @@@ paradedb.all()
    GROUP BY account_id
) AS grouped
ORDER BY 1;

SELECT outer_values.account_id AS outer_account_id, grouped.*
FROM (VALUES (1::bigint), (2), (99)) AS outer_values (account_id),
LATERAL (
    SELECT account_id, COUNT(*)
    FROM pinned_key_items
    WHERE account_id = outer_values.account_id AND id @@@ paradedb.all()
    GROUP BY account_id
) AS grouped
ORDER BY 1;

-- =====================================================================
-- SECTION 5: Equal values that are not always identical
-- =====================================================================
-- The rows that are equal to the constant can hold different values (`-0` and
-- `0`). The scan reads the key from a row, as PostgreSQL does.

\echo 'Test 5.1: float key'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT price, kind, COUNT(*)
FROM pinned_key_items
WHERE price = 1 AND id @@@ paradedb.all()
GROUP BY price, kind
ORDER BY kind;

SELECT price, kind, COUNT(*)
FROM pinned_key_items
WHERE price = 1 AND id @@@ paradedb.all()
GROUP BY price, kind
ORDER BY kind;

\echo 'Test 5.2: key with a nondeterministic collation -> declined'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT label, kind, COUNT(*)
FROM pinned_key_items
WHERE label = 'label' AND id @@@ paradedb.all()
GROUP BY label, kind
ORDER BY kind;

SELECT label, kind, COUNT(*)
FROM pinned_key_items
WHERE label = 'label' AND id @@@ paradedb.all()
GROUP BY label, kind
ORDER BY kind;

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT label, COUNT(*)
FROM pinned_key_items
WHERE label = 'label' AND id @@@ paradedb.all()
GROUP BY label;

SELECT label, COUNT(*)
FROM pinned_key_items
WHERE label = 'label' AND id @@@ paradedb.all()
GROUP BY label;

\echo 'Test 5.3: constant of a type that the key is cast to for the comparison'
-- Two `timestamp` values can be equal to one `timestamptz` constant at a
-- daylight saving change.
SET timezone = 'UTC';

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT updated, kind, COUNT(*)
FROM pinned_key_items
WHERE updated = TIMESTAMPTZ '2024-01-02 00:00:00+00' AND id @@@ paradedb.all()
GROUP BY updated, kind
ORDER BY kind;

SELECT updated, kind, COUNT(*)
FROM pinned_key_items
WHERE updated = TIMESTAMPTZ '2024-01-02 00:00:00+00' AND id @@@ paradedb.all()
GROUP BY updated, kind
ORDER BY kind;

RESET timezone;

\echo 'Test 5.4: value of a stable expression'
SET pinned_key.account = '1';

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = current_setting('pinned_key.account')::bigint AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = current_setting('pinned_key.account')::bigint AND id @@@ paradedb.all()
GROUP BY account_id;

RESET pinned_key.account;

\echo 'Test 5.6: two stored values are equal to one constant'
-- At a daylight saving change, two `timestamp` values are equal to one
-- `timestamptz` constant. PostgreSQL has one group, and reads the key from one
-- of its rows.
CREATE TABLE pinned_key_dst (id SERIAL PRIMARY KEY, ts TIMESTAMP, amount NUMERIC(10, 2), kind TEXT);
INSERT INTO pinned_key_dst (ts, amount, kind) VALUES
    ('2024-03-10 02:30', 1.5, 'a'),
    ('2024-03-10 03:30', 2.5, 'a'),
    ('2024-03-10 04:30', 4, 'b');
CREATE INDEX pinned_key_dst_idx ON pinned_key_dst USING paradedb (id, ts, amount, (kind::pdb.literal));
SET timezone = 'America/New_York';

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT ts, SUM(amount)
FROM pinned_key_dst
WHERE ts = TIMESTAMPTZ '2024-03-10 07:30:00+00' AND id @@@ paradedb.all()
GROUP BY ts;

SELECT ts, SUM(amount)
FROM pinned_key_dst
WHERE ts = TIMESTAMPTZ '2024-03-10 07:30:00+00' AND id @@@ paradedb.all()
GROUP BY ts;

SELECT ts, kind, SUM(amount)
FROM pinned_key_dst
WHERE ts = TIMESTAMPTZ '2024-03-10 07:30:00+00' AND id @@@ paradedb.all()
GROUP BY ts, kind;

RESET timezone;

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT price, kind, SUM(amount)
FROM pinned_key_items
WHERE price = 1 AND id @@@ paradedb.all()
GROUP BY price, kind
ORDER BY kind;

SELECT price, kind, SUM(amount)
FROM pinned_key_items
WHERE price = 1 AND id @@@ paradedb.all()
GROUP BY price, kind
ORDER BY kind;

\echo 'Test 5.5: pdb.agg()'
SELECT price, kind, pdb.agg('{"value_count": {"field": "id"}}'::jsonb)
FROM pinned_key_items
WHERE price = 1 AND id @@@ paradedb.all()
GROUP BY price, kind;

-- =====================================================================
-- SECTION 6: NUMERIC keys and aggregates
-- =====================================================================

\echo 'Test 6.1: the only key is pinned'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, SUM(amount)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, SUM(amount)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, SUM(amount)
FROM pinned_key_items
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 6.2: pinned NUMERIC key'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT amount, COUNT(*)
FROM pinned_key_items
WHERE amount = 1.5 AND id @@@ paradedb.all()
GROUP BY amount;

SELECT amount, COUNT(*)
FROM pinned_key_items
WHERE amount = 1.5 AND id @@@ paradedb.all()
GROUP BY amount;

\echo 'Test 6.3: pinned key with a nondeterministic collation -> declined'
-- The collation can call different bytes equal.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT kind, SUM(amount)
FROM pinned_key_items
WHERE label = 'label' AND id <= 12 AND id @@@ paradedb.all()
GROUP BY label, kind
ORDER BY kind;

SELECT kind, SUM(amount)
FROM pinned_key_items
WHERE label = 'label' AND id <= 12 AND id @@@ paradedb.all()
GROUP BY label, kind
ORDER BY kind;

-- =====================================================================
-- SECTION 6b: The pin and the aggregate are in a nested query
-- =====================================================================
-- PostgreSQL plans each query level on its own, and the scan sees the level
-- that holds the aggregate.

\echo 'Test 6b.1: the grouped subquery is joined to another table'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT grouped.account_id, grouped.kind, grouped.n
FROM (
    SELECT account_id, kind, COUNT(*) AS n
    FROM pinned_key_items
    WHERE account_id = 2 AND id @@@ paradedb.all()
    GROUP BY account_id, kind
) AS grouped
JOIN (VALUES (1::bigint), (2), (3)) AS accounts (id) ON accounts.id = grouped.account_id
ORDER BY grouped.kind;

SELECT grouped.account_id, grouped.kind, grouped.n
FROM (
    SELECT account_id, kind, COUNT(*) AS n
    FROM pinned_key_items
    WHERE account_id = 2 AND id @@@ paradedb.all()
    GROUP BY account_id, kind
) AS grouped
JOIN (VALUES (1::bigint), (2), (3)) AS accounts (id) ON accounts.id = grouped.account_id
ORDER BY grouped.kind;

\echo 'Test 6b.2: an outer aggregate over the pinned aggregate'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT kind, SUM(n), COUNT(*)
FROM (
    SELECT account_id, kind, COUNT(*) AS n
    FROM pinned_key_items
    WHERE account_id = 2 AND id @@@ paradedb.all()
    GROUP BY account_id, kind
) AS grouped
GROUP BY kind
ORDER BY kind;

SELECT kind, SUM(n), COUNT(*)
FROM (
    SELECT account_id, kind, COUNT(*) AS n
    FROM pinned_key_items
    WHERE account_id = 2 AND id @@@ paradedb.all()
    GROUP BY account_id, kind
) AS grouped
GROUP BY kind
ORDER BY kind;

\echo 'Test 6b.3: a materialized CTE keeps the outer condition out of the aggregate'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
WITH grouped AS MATERIALIZED (
    SELECT account_id, kind, COUNT(*) AS n
    FROM pinned_key_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id, kind
)
SELECT * FROM grouped WHERE account_id = 2 ORDER BY kind;

WITH grouped AS MATERIALIZED (
    SELECT account_id, kind, COUNT(*) AS n
    FROM pinned_key_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id, kind
)
SELECT * FROM grouped WHERE account_id = 2 ORDER BY kind;

\echo 'Test 6b.4: IN with one value'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, kind, COUNT(*)
FROM pinned_key_items
WHERE account_id IN (2) AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT account_id, kind, COUNT(*)
FROM pinned_key_items
WHERE account_id IN (2) AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

\echo 'Test 6b.5: two different constants -> no row'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, kind, COUNT(*)
FROM pinned_key_items
WHERE account_id = 2 AND account_id = 3 AND id @@@ paradedb.all()
GROUP BY account_id, kind;

SELECT account_id, kind, COUNT(*)
FROM pinned_key_items
WHERE account_id = 2 AND account_id = 3 AND id @@@ paradedb.all()
GROUP BY account_id, kind;

-- =====================================================================
-- SECTION 7: Same results from PostgreSQL
-- =====================================================================

SET paradedb.enable_aggregate_custom_scan TO off;

SET timezone = 'America/New_York';

SELECT ts, SUM(amount)
FROM pinned_key_dst
WHERE ts = TIMESTAMPTZ '2024-03-10 07:30:00+00' AND id @@@ paradedb.all()
GROUP BY ts;

SELECT ts, kind, SUM(amount)
FROM pinned_key_dst
WHERE ts = TIMESTAMPTZ '2024-03-10 07:30:00+00' AND id @@@ paradedb.all()
GROUP BY ts, kind;

RESET timezone;

SELECT price, kind, SUM(amount)
FROM pinned_key_items
WHERE price = 1 AND id @@@ paradedb.all()
GROUP BY price, kind
ORDER BY kind;

SELECT grouped.account_id, grouped.kind, grouped.n
FROM (
    SELECT account_id, kind, COUNT(*) AS n
    FROM pinned_key_items
    WHERE account_id = 2 AND id @@@ paradedb.all()
    GROUP BY account_id, kind
) AS grouped
JOIN (VALUES (1::bigint), (2), (3)) AS accounts (id) ON accounts.id = grouped.account_id
ORDER BY grouped.kind;

SELECT kind, SUM(n), COUNT(*)
FROM (
    SELECT account_id, kind, COUNT(*) AS n
    FROM pinned_key_items
    WHERE account_id = 2 AND id @@@ paradedb.all()
    GROUP BY account_id, kind
) AS grouped
GROUP BY kind
ORDER BY kind;

WITH grouped AS MATERIALIZED (
    SELECT account_id, kind, COUNT(*) AS n
    FROM pinned_key_items
    WHERE id @@@ paradedb.all()
    GROUP BY account_id, kind
)
SELECT * FROM grouped WHERE account_id = 2 ORDER BY kind;

SELECT account_id, kind, COUNT(*)
FROM pinned_key_items
WHERE account_id IN (2) AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT account_id, kind, COUNT(*)
FROM pinned_key_items
WHERE account_id = 2 AND account_id = 3 AND id @@@ paradedb.all()
GROUP BY account_id, kind;

SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT code, account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 2 AND code = 'c1' AND id @@@ paradedb.all()
GROUP BY account_id, code;

SELECT COUNT(*) AS count_all, account_id, kind
FROM pinned_key_items
WHERE region = 1 AND account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT account_id, kind, SUM(price)
FROM pinned_key_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY account_id, kind
LIMIT 3;

SELECT account_id, kind, COALESCE(SUM(price), 0) + 1 AS total
FROM pinned_key_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY account_id, kind
ORDER BY kind;

SELECT region, COUNT(*)
FROM pinned_key_items
WHERE region = 2::bigint AND id @@@ paradedb.all()
GROUP BY region;

SELECT region, COUNT(*)
FROM pinned_key_items
WHERE region = 100000 AND id @@@ paradedb.all()
GROUP BY region;

SELECT code, kind, COUNT(*)
FROM pinned_key_items
WHERE code = 'c1' AND id @@@ paradedb.all()
GROUP BY code, kind
ORDER BY kind;

SELECT created, COUNT(*)
FROM pinned_key_items
WHERE created = '2024-01-02' AND id @@@ paradedb.all()
GROUP BY created;

SELECT metadata->>'color' AS color, kind, COUNT(*)
FROM pinned_key_items
WHERE metadata->>'color' = 'red' AND id @@@ paradedb.all()
GROUP BY metadata->>'color', kind
ORDER BY kind;

SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = (SELECT 3) AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT outer_values.account_id AS outer_account_id, grouped.*
FROM (VALUES (1::bigint), (2), (99)) AS outer_values (account_id),
LATERAL (
    SELECT account_id, COUNT(*)
    FROM pinned_key_items
    WHERE account_id = outer_values.account_id AND id @@@ paradedb.all()
    GROUP BY account_id
) AS grouped
ORDER BY 1;

SELECT account_id, SUM(amount)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT amount, COUNT(*)
FROM pinned_key_items
WHERE amount = 1.5 AND id @@@ paradedb.all()
GROUP BY amount;

SELECT account_id, COUNT(*) FILTER (WHERE kind = 'none')
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*) FILTER (WHERE kind = 'none')
FROM pinned_key_items
WHERE account_id = 99 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT id, account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 3;

SELECT upper(kind) AS upper_kind, COUNT(*)
FROM pinned_key_items
WHERE upper(kind) = 'A' AND id @@@ paradedb.all()
GROUP BY upper(kind);

SELECT DISTINCT account_id, COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*), SUM(COUNT(*)) OVER ()
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, generate_series(1, 2) AS n, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(id ORDER BY kind)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, code, COUNT(id ORDER BY kind), SUM(price ORDER BY kind)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id, code
ORDER BY code;

-- =====================================================================
-- SECTION 8: Deleted rows
-- =====================================================================
-- The index still has the documents of the deleted rows. They must not make
-- a group.

DELETE FROM pinned_key_items WHERE account_id = 1;
DELETE FROM pinned_key_items WHERE account_id = 2 AND kind <> 'a';

SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY account_id;

SET paradedb.enable_aggregate_custom_scan TO on;

\echo 'Test 8.1: every matching row is deleted -> no group'
SELECT account_id, COUNT(*)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

SELECT account_id, COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 1 AND id @@@ paradedb.all()
GROUP BY account_id;

\echo 'Test 8.2: some matching rows are deleted'
SELECT account_id, COUNT(*), SUM(price)
FROM pinned_key_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY account_id;

RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE pinned_key_items;
DROP TABLE pinned_key_dst;
DROP COLLATION pinned_key_case_insensitive;

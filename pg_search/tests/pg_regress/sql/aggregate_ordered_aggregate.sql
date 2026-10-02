-- An aggregate with its own ORDER BY, such as `COUNT(id ORDER BY kind)`.
-- On PG16 and later, PostgreSQL puts the sort keys of such an aggregate after
-- the GROUP BY keys in `group_pathkeys`. The Aggregate Scan must not group on
-- them.

\i common/common_setup.sql

CREATE COLLATION IF NOT EXISTS ordered_agg_case_insensitive (
    provider = icu,
    locale = 'und-u-ks-level2',
    deterministic = false
);

CREATE TABLE ordered_agg_items (
    id SERIAL PRIMARY KEY,
    account_id BIGINT,
    kind TEXT,
    price FLOAT8,
    amount NUMERIC(10, 2)
);

INSERT INTO ordered_agg_items (account_id, kind, price, amount)
SELECT (g % 3) + 1, (ARRAY['a', 'b', 'c', 'd'])[(g % 4) + 1], g % 5, (g % 5) + 0.5
FROM generate_series(1, 120) g;

CREATE INDEX ordered_agg_items_idx ON ordered_agg_items
USING paradedb (id, account_id, (kind::pdb.literal), price, amount)
WITH (key_field = 'id');

SET paradedb.enable_aggregate_custom_scan TO on;

\echo 'Test 1: no GROUP BY -> one row'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(id ORDER BY kind)
FROM ordered_agg_items
WHERE id @@@ paradedb.all();

SELECT COUNT(id ORDER BY kind)
FROM ordered_agg_items
WHERE id @@@ paradedb.all();

\echo 'Test 2: GROUP BY -> one row for each group'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, COUNT(id ORDER BY kind)
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

SELECT account_id, COUNT(id ORDER BY kind)
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

\echo 'Test 3: other aggregates, and a sort key that is not in the index'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT account_id, SUM(price ORDER BY kind), MIN(price ORDER BY kind), MAX(price ORDER BY upper(kind))
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

SELECT account_id, SUM(price ORDER BY kind), MIN(price ORDER BY kind), MAX(price ORDER BY upper(kind))
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

\echo 'Test 4: the sort key is also a GROUP BY key'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT kind, COUNT(id ORDER BY kind, account_id)
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY kind
ORDER BY kind;

SELECT kind, COUNT(id ORDER BY kind, account_id)
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY kind
ORDER BY kind;

\echo 'Test 5: DISTINCT aggregate -> declined'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT account_id, COUNT(DISTINCT kind)
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

SELECT account_id, COUNT(DISTINCT kind)
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

\echo 'Test 6: pdb.agg() -> one row'
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb ORDER BY kind)
FROM ordered_agg_items
WHERE id @@@ paradedb.all();

\echo 'Test 7: DataFusion backend, sort key with a nondeterministic collation -> declined'
-- The NUMERIC aggregate routes the query to DataFusion, which sorts by the
-- bytes of the key.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT account_id, SUM(amount), string_agg(kind, ',' ORDER BY kind COLLATE ordered_agg_case_insensitive)
FROM ordered_agg_items
WHERE id <= 12 AND id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

SELECT account_id, SUM(amount), string_agg(kind, ',' ORDER BY kind COLLATE ordered_agg_case_insensitive)
FROM ordered_agg_items
WHERE id <= 12 AND id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT SUM(amount), COUNT(DISTINCT kind COLLATE ordered_agg_case_insensitive)
FROM ordered_agg_items
WHERE id @@@ paradedb.all();

SELECT SUM(amount), COUNT(DISTINCT kind COLLATE ordered_agg_case_insensitive)
FROM ordered_agg_items
WHERE id @@@ paradedb.all();

\echo 'Same results from PostgreSQL'
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT COUNT(id ORDER BY kind)
FROM ordered_agg_items
WHERE id @@@ paradedb.all();

SELECT account_id, COUNT(id ORDER BY kind)
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

SELECT account_id, SUM(price ORDER BY kind), MIN(price ORDER BY kind), MAX(price ORDER BY upper(kind))
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

SELECT kind, COUNT(id ORDER BY kind, account_id)
FROM ordered_agg_items
WHERE id @@@ paradedb.all()
GROUP BY kind
ORDER BY kind;

SELECT account_id, SUM(amount), string_agg(kind, ',' ORDER BY kind COLLATE ordered_agg_case_insensitive)
FROM ordered_agg_items
WHERE id <= 12 AND id @@@ paradedb.all()
GROUP BY account_id
ORDER BY account_id;

SELECT SUM(amount), COUNT(DISTINCT kind COLLATE ordered_agg_case_insensitive)
FROM ordered_agg_items
WHERE id @@@ paradedb.all();

RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE ordered_agg_items;
DROP COLLATION ordered_agg_case_insensitive;

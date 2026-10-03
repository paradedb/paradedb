-- A column that the query returns and that PostgreSQL does not group on.
-- PostgreSQL takes such a column when the keys decide its value: they have the
-- primary key of its table. It also drops such a column from the keys when the
-- query has it in the GROUP BY. The Aggregate Scan must return its value.

\i common/common_setup.sql

CREATE TABLE dependent_col_items (
    id SERIAL PRIMARY KEY,
    account_id BIGINT,
    region INT,
    kind TEXT,
    price FLOAT8,
    created DATE,
    tags TEXT[],
    meta JSONB,
    note TEXT
);

INSERT INTO dependent_col_items (account_id, region, kind, price, created, tags, meta, note)
SELECT
    CASE WHEN g % 7 = 0 THEN NULL ELSE (g % 3) + 1 END,
    (g % 5) + 1,
    CASE WHEN g % 5 = 0 THEN NULL ELSE (ARRAY['a', 'b', 'c', 'd'])[(g % 4) + 1] END,
    CASE WHEN g % 6 = 0 THEN NULL ELSE g % 5 END,
    CASE WHEN g % 4 = 0 THEN NULL ELSE DATE '2024-01-01' + g END,
    CASE g % 4 WHEN 0 THEN NULL WHEN 1 THEN '{}'::text[] WHEN 2 THEN ARRAY[NULL, 'x'] ELSE ARRAY['x', 'y'] END,
    jsonb_build_object('n', g),
    'note ' || g
FROM generate_series(1, 40) g;

-- The largest and the smallest value of the type must come back as they are.
UPDATE dependent_col_items SET account_id = 9223372036854775807 WHERE id = 1;
UPDATE dependent_col_items SET account_id = -9223372036854775808 WHERE id = 2;

-- `note` is not in the index.
CREATE INDEX dependent_col_items_idx ON dependent_col_items
USING paradedb (id, account_id, region, (kind::pdb.literal), price, created, (tags::pdb.literal), meta);

SET paradedb.enable_aggregate_custom_scan TO on;

\echo 'Test 1: two keys, and no key decides the other: each pair of values is a group'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT price, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY price, kind
ORDER BY price, kind;

SELECT price, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY price, kind
ORDER BY price, kind;

-- The WHERE clause pins one key. The other key still decides the groups.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT region, kind, COUNT(*)
FROM dependent_col_items
WHERE region = 2 AND id @@@ paradedb.all()
GROUP BY region, kind
ORDER BY kind;

SELECT region, kind, COUNT(*)
FROM dependent_col_items
WHERE region = 2 AND id @@@ paradedb.all()
GROUP BY region, kind
ORDER BY kind;

\echo 'Test 2: the column is in the GROUP BY, and PostgreSQL drops it from the keys'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id, kind
ORDER BY id, kind
LIMIT 6;

SELECT id, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id, kind
ORDER BY id, kind
LIMIT 6;

\echo 'Test 3: the columns are not in the GROUP BY'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, kind, account_id, price, created, COUNT(*), SUM(price)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 8;

SELECT id, kind, account_id, price, created, COUNT(*), SUM(price)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 8;

\echo 'Test 4: a cast of the column next to the column'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, created, created::text AS created_text, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

SELECT id, created, created::text AS created_text, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

\echo 'Test 5: ORDER BY and HAVING on the column'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
HAVING kind <> 'a'
ORDER BY kind DESC, id
LIMIT 5;

SELECT id, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
HAVING kind <> 'a'
ORDER BY kind DESC, id
LIMIT 5;

\echo 'Test 6: LIMIT with no ORDER BY'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
LIMIT 3;

-- The rows of a LIMIT with no ORDER BY are not stable, so this compares each
-- row with the table.
SELECT limited.kind IS NOT DISTINCT FROM (
    SELECT t.kind FROM dependent_col_items t WHERE t.id = limited.id
) AS same_kind
FROM (
    SELECT id, kind, COUNT(*)
    FROM dependent_col_items
    WHERE id @@@ paradedb.all()
    GROUP BY id
    LIMIT 3
) AS limited;

\echo 'Test 7: the primary key is pinned to one value'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, kind, account_id, COUNT(*)
FROM dependent_col_items
WHERE id = 8 AND id @@@ paradedb.all()
GROUP BY id;

SELECT id, kind, account_id, COUNT(*)
FROM dependent_col_items
WHERE id = 8 AND id @@@ paradedb.all()
GROUP BY id;

SELECT id, kind, account_id, COUNT(*)
FROM dependent_col_items
WHERE id = 999 AND id @@@ paradedb.all()
GROUP BY id;

\echo 'Test 8: the column is pinned to one value, and it is not a key'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, account_id, COUNT(*)
FROM dependent_col_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

SELECT id, account_id, COUNT(*)
FROM dependent_col_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

-- A float column: equal values are not always identical, and the value comes
-- from the row.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, price, COUNT(*)
FROM dependent_col_items
WHERE price = 2 AND id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

SELECT id, price, COUNT(*)
FROM dependent_col_items
WHERE price = 2 AND id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

\echo 'Test 9: pdb.agg() next to the column'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, kind, pdb.agg('{"value_count": {"field": "price"}}'::jsonb)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 6;

SELECT id, kind, pdb.agg('{"value_count": {"field": "price"}}'::jsonb)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 6;

\echo 'Test 10: a parameter in a plan that PostgreSQL uses again'
SET plan_cache_mode TO force_generic_plan;
PREPARE dependent_col_by_id(int) AS
SELECT id, kind, created, COUNT(*)
FROM dependent_col_items
WHERE id = $1 AND id @@@ paradedb.all()
GROUP BY id;

EXECUTE dependent_col_by_id(3);
EXECUTE dependent_col_by_id(4);
EXECUTE dependent_col_by_id(999);

DEALLOCATE dependent_col_by_id;
RESET plan_cache_mode;

\echo 'Test 11: an array column -> declined, because the index does not hold the empty array or a NULL element'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, tags, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

SELECT id, tags, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

\echo 'Test 12: a key that is an expression, next to the column -> declined'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id, created::text
ORDER BY id
LIMIT 3;

SELECT id, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id, created::text
ORDER BY id
LIMIT 3;

\echo 'Test 13: a key that is a cast, with a window function above it -> declined'
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT price::text || '!' AS label, COUNT(*) AS n, SUM(COUNT(*)) OVER () AS total
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY price::text
ORDER BY 1;

SELECT price::text || '!' AS label, COUNT(*) AS n, SUM(COUNT(*)) OVER () AS total
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY price::text
ORDER BY 1;

\echo 'Test 14: the column is not columnar -> declined'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, meta, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 3;

SELECT id, meta, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 3;

\echo 'Test 15: the column is not in the index -> declined'
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, note, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 3;

SELECT id, note, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 3;

\echo 'Test 16: a window over an expression of a DATE() key reads the timestamp from a row'
CREATE TABLE dependent_col_days (id INT PRIMARY KEY, ts TIMESTAMP);
INSERT INTO dependent_col_days
SELECT g, TIMESTAMP '2024-01-01 00:00' + (g * INTERVAL '7 hours') FROM generate_series(1, 30) g;
CREATE INDEX dependent_col_days_idx ON dependent_col_days USING paradedb (id, ts);

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT date(ts)::text AS d, COUNT(*) AS n, SUM(COUNT(*)) OVER () AS total
FROM dependent_col_days
WHERE id @@@ paradedb.all()
GROUP BY date(ts)
ORDER BY 1;

SELECT date(ts)::text AS d, COUNT(*) AS n, SUM(COUNT(*)) OVER () AS total
FROM dependent_col_days
WHERE id @@@ paradedb.all()
GROUP BY date(ts)
ORDER BY 1;

\echo 'Test 17: a key that is a cast, on the DataFusion backend -> declined'
SET paradedb.max_term_agg_buckets TO 2;

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT account_id::text AS account, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY account_id::text
ORDER BY 1;

SELECT account_id::text AS account, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY account_id::text
ORDER BY 1;

RESET paradedb.max_term_agg_buckets;

\echo 'Same results from PostgreSQL'
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT price, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY price, kind
ORDER BY price, kind;

SELECT region, kind, COUNT(*)
FROM dependent_col_items
WHERE region = 2 AND id @@@ paradedb.all()
GROUP BY region, kind
ORDER BY kind;

SELECT id, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id, kind
ORDER BY id, kind
LIMIT 6;

SELECT id, kind, account_id, price, created, COUNT(*), SUM(price)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 8;

SELECT id, created, created::text AS created_text, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

SELECT id, kind, COUNT(*)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
HAVING kind <> 'a'
ORDER BY kind DESC, id
LIMIT 5;

SELECT price::text || '!' AS label, COUNT(*) AS n, SUM(COUNT(*)) OVER () AS total
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY price::text
ORDER BY 1;

SELECT id, kind, account_id, COUNT(*)
FROM dependent_col_items
WHERE id = 8 AND id @@@ paradedb.all()
GROUP BY id;

SELECT id, kind, account_id, COUNT(*)
FROM dependent_col_items
WHERE id = 999 AND id @@@ paradedb.all()
GROUP BY id;

SELECT id, account_id, COUNT(*)
FROM dependent_col_items
WHERE account_id = 2 AND id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

SELECT id, price, COUNT(*)
FROM dependent_col_items
WHERE price = 2 AND id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 4;

-- `pdb.agg()` needs the scan, so `COUNT` is its twin.
SELECT id, kind, COUNT(price)
FROM dependent_col_items
WHERE id @@@ paradedb.all()
GROUP BY id
ORDER BY id
LIMIT 6;

SET plan_cache_mode TO force_generic_plan;
PREPARE dependent_col_by_id(int) AS
SELECT id, kind, created, COUNT(*)
FROM dependent_col_items
WHERE id = $1 AND id @@@ paradedb.all()
GROUP BY id;

EXECUTE dependent_col_by_id(3);
EXECUTE dependent_col_by_id(4);
EXECUTE dependent_col_by_id(999);

DEALLOCATE dependent_col_by_id;
RESET plan_cache_mode;

SELECT date(ts)::text AS d, COUNT(*) AS n, SUM(COUNT(*)) OVER () AS total
FROM dependent_col_days
WHERE id @@@ paradedb.all()
GROUP BY date(ts)
ORDER BY 1;

RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE dependent_col_items;
DROP TABLE dependent_col_days;

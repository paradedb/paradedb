-- =====================================================================
-- Issue #5573: LIMIT must not be pushed into BaseScan when a
--              row-reducing set-returning function sits above it.
-- =====================================================================
-- Before the fix, `classify_target_list_srf` marked `unnest` as
-- "row-preserving" and let BaseScan cap its TopK to `LIMIT` rows.
-- `unnest(ARRAY[]::T[])` and `unnest(NULL)` emit ZERO rows, so the
-- ProjectSet above the scan then discarded rows from an already-capped
-- output and returned fewer than LIMIT results.
--
-- This test exercises the buggy case (mixed empty/NULL/non-empty
-- arrays under `unnest` with a `@@@` predicate + LIMIT) and asserts
-- the query returns exactly LIMIT rows.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP SCHEMA IF EXISTS pdb_unnest_limit CASCADE;
CREATE SCHEMA pdb_unnest_limit;
SET search_path = pdb_unnest_limit, public;

CREATE TABLE items (id int PRIMARY KEY, kind text);
INSERT INTO items
SELECT g, CASE WHEN g % 3 = 0 THEN 'novel' ELSE 'manga' END
FROM generate_series(1, 2000) g;
CREATE INDEX items_bm25 ON items USING bm25 (id, kind) WITH (key_field = 'id');
ANALYZE items;

SET paradedb.enable_custom_scan = on;

-- Row-reducing unnest: even ids emit one row, odd ids emit zero rows.
-- Correct answer is 5 rows because there are >>5 matching even ids.
-- Before the fix, this returned only 2 rows: TopK capped input to 5
-- (ids 1, 2, 4, 5, 7), then ProjectSet dropped ids 1, 5, 7.
SELECT id, unnest(CASE WHEN id % 2 = 0 THEN ARRAY[1] ELSE '{}'::int[] END) AS u
FROM items
WHERE kind @@@ pdb.term('manga')
ORDER BY id
LIMIT 5;

-- Same shape with NULL arrays instead of empty arrays.
SELECT id, unnest(CASE WHEN id % 2 = 0 THEN ARRAY[1] ELSE NULL::int[] END) AS u
FROM items
WHERE kind @@@ pdb.term('manga')
ORDER BY id
LIMIT 5;

-- Sanity: the non-reducing case (every input row multiplies into >=1 output
-- rows) still returns exactly LIMIT rows and still routes through the scan.
SELECT id, unnest(ARRAY[1, 2]) AS u
FROM items
WHERE kind @@@ pdb.term('manga')
ORDER BY id, u
LIMIT 6;

-- Sanity: with `paradedb.enable_custom_scan = off` the query falls back to
-- a plain PostgreSQL plan (no BaseScan involved) and should return 5 rows.
-- This is the reference the fix restores parity with.
SET paradedb.enable_custom_scan = off;
SELECT id, unnest(CASE WHEN id % 2 = 0 THEN ARRAY[1] ELSE '{}'::int[] END) AS u
FROM items
WHERE kind = 'manga'
ORDER BY id
LIMIT 5;

RESET paradedb.enable_custom_scan;
DROP SCHEMA pdb_unnest_limit CASCADE;

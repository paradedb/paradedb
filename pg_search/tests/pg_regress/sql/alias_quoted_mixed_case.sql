-- =====================================================================
-- Issue #5525: DataFusion identifier normalization on quoted mixed-case
-- table aliases (aggregate custom scan AND join custom scan)
-- =====================================================================
-- The alias emitted by `RelationAlias::execution` was preserving the
-- original case of a Postgres alias (e.g. `parent AS "Parent"` -> "Parent_0")
-- while downstream schema registration (`register_table` / `df.alias`) went
-- through DataFusion's `parse_str` and lower-cased unquoted identifiers
-- to `parent_0`. Column references built via `TableReference::Bare` in
-- `make_col` kept the original case, so the schema said `parent_0` while
-- the reference said `"Parent_0"`, yielding `FieldNotFound`.
--
-- Both custom scans (aggregate and join) go through the same
-- `RelationAlias::execution` emitter, so both are covered by the fix.
--
-- Each test has three parts:
--   1. EXPLAIN to confirm the ParadeDB custom scan is used (not fallback).
--   2. Run the query with the custom scan on -- the path this fix covers.
--   3. Run the same query with the custom scan off -- native Postgres parity.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test Data
CREATE TABLE repro_5525_parent (
    id         bigint      PRIMARY KEY,
    child_id   bigint      NOT NULL,
    owner      text        NOT NULL,
    updated_at timestamptz NOT NULL
);
CREATE TABLE repro_5525_child (
    id    bigint PRIMARY KEY,
    state text NOT NULL
);

INSERT INTO repro_5525_child
SELECT g, CASE WHEN g % 5 = 0 THEN 'inactive' ELSE 'active' END
FROM generate_series(1, 10) g;

INSERT INTO repro_5525_parent
SELECT g,
       ((g % 10) + 1),
       'user-1',
       '2024-01-01 00:00:00+00'::timestamptz + (g || ' hours')::interval
FROM generate_series(1, 10) g;

CREATE INDEX repro_5525_parent_idx ON repro_5525_parent
USING bm25 (id, child_id, (owner::pdb.literal), updated_at)
WITH (key_field='id');

CREATE INDEX repro_5525_child_idx ON repro_5525_child
USING bm25 (id, (state::pdb.literal))
WITH (key_field='id');

ANALYZE repro_5525_parent;
ANALYZE repro_5525_child;

-- =====================================================================
-- Trigger 1 -- aggregate custom scan
-- =====================================================================
-- count("Parent"."id") over a join with quoted mixed-case aliases.
-- No @@@ operator; the aggregate custom scan intercepts purely because
-- the tables carry BM25 indexes.
-- Previously crashed with `No field named "Parent_0".id`.

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT count("Parent"."id")
FROM repro_5525_parent AS "Parent"
JOIN repro_5525_child  AS "Child"
  ON "Parent"."child_id" = "Child"."id" AND "Child"."state" = 'active'
WHERE "Parent"."owner" = 'user-1';

SELECT count("Parent"."id")
FROM repro_5525_parent AS "Parent"
JOIN repro_5525_child  AS "Child"
  ON "Parent"."child_id" = "Child"."id" AND "Child"."state" = 'active'
WHERE "Parent"."owner" = 'user-1';

-- Parity: same query with aggregate custom scan off (native Postgres).
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT count("Parent"."id")
FROM repro_5525_parent AS "Parent"
JOIN repro_5525_child  AS "Child"
  ON "Parent"."child_id" = "Child"."id" AND "Child"."state" = 'active'
WHERE "Parent"."owner" = 'user-1';
SET paradedb.enable_aggregate_custom_scan TO on;

-- Control: identical query with lowercase (unquoted) aliases -- always worked.
SELECT count(parent.id)
FROM repro_5525_parent AS parent
JOIN repro_5525_child  AS child
  ON parent.child_id = child.id AND child.state = 'active'
WHERE parent.owner = 'user-1';

-- =====================================================================
-- Trigger 2 -- join custom scan
-- =====================================================================
-- @@@ predicate on one side + ORDER BY a fast field on the other side,
-- with quoted mixed-case aliases.
-- Previously crashed with `FieldNotFound "Child_0".ctid_0`.

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT "Parent"."id"
FROM repro_5525_parent AS "Parent"
JOIN repro_5525_child  AS "Child"
  ON "Parent"."child_id" = "Child"."id"
WHERE "Parent"."owner" = 'user-1'
  AND "Child"."id" @@@ paradedb.term('state', 'active'::text)
ORDER BY "Parent"."updated_at" DESC
LIMIT 12;

SELECT "Parent"."id"
FROM repro_5525_parent AS "Parent"
JOIN repro_5525_child  AS "Child"
  ON "Parent"."child_id" = "Child"."id"
WHERE "Parent"."owner" = 'user-1'
  AND "Child"."id" @@@ paradedb.term('state', 'active'::text)
ORDER BY "Parent"."updated_at" DESC
LIMIT 12;

-- Parity: same query with join custom scan off (native Postgres).
SET paradedb.enable_join_custom_scan TO off;
SELECT "Parent"."id"
FROM repro_5525_parent AS "Parent"
JOIN repro_5525_child  AS "Child"
  ON "Parent"."child_id" = "Child"."id"
WHERE "Parent"."owner" = 'user-1'
  AND "Child"."id" @@@ paradedb.term('state', 'active'::text)
ORDER BY "Parent"."updated_at" DESC
LIMIT 12;
SET paradedb.enable_join_custom_scan TO on;

-- Control: identical query with lowercase (unquoted) aliases -- always worked.
SELECT parent.id
FROM repro_5525_parent AS parent
JOIN repro_5525_child  AS child
  ON parent.child_id = child.id
WHERE parent.owner = 'user-1'
  AND child.id @@@ paradedb.term('state', 'active'::text)
ORDER BY parent.updated_at DESC
LIMIT 12;

-- =====================================================================
-- Trigger 3 -- quoted aliases with characters bare identifiers cannot use
-- =====================================================================
-- PostgreSQL permits quoted table aliases such as `"P.A"` and `"123"` that
-- are not valid bare identifiers. Custom scans must treat each of these as
-- a single relation and produce the same result as the corresponding
-- lowercase-aliased query.

SELECT count("P.A"."id")
FROM repro_5525_parent AS "P.A"
JOIN repro_5525_child  AS "C.A"
  ON "P.A"."child_id" = "C.A"."id" AND "C.A"."state" = 'active'
WHERE "P.A"."owner" = 'user-1';

SELECT "P.A"."id"
FROM repro_5525_parent AS "P.A"
JOIN repro_5525_child  AS "C.A"
  ON "P.A"."child_id" = "C.A"."id"
WHERE "P.A"."owner" = 'user-1'
  AND "C.A"."id" @@@ paradedb.term('state', 'active'::text)
ORDER BY "P.A"."updated_at" DESC
LIMIT 12;

-- Numeric-only aliases (leading digit) must also work.
SELECT count("123"."id")
FROM repro_5525_parent AS "123"
JOIN repro_5525_child  AS "456"
  ON "123"."child_id" = "456"."id" AND "456"."state" = 'active'
WHERE "123"."owner" = 'user-1';

SELECT "123"."id"
FROM repro_5525_parent AS "123"
JOIN repro_5525_child  AS "456"
  ON "123"."child_id" = "456"."id"
WHERE "123"."owner" = 'user-1'
  AND "456"."id" @@@ paradedb.term('state', 'active'::text)
ORDER BY "123"."updated_at" DESC
LIMIT 12;

-- Cleanup
DROP TABLE repro_5525_parent CASCADE;
DROP TABLE repro_5525_child CASCADE;

-- Regression test: partial-index gating across every bm25 custom scan.
--
-- A partial bm25 index only contains rows satisfying its WHERE predicate. A
-- custom scan may push a query down to it only when the query's clauses imply
-- that predicate; otherwise it would answer from an index that is missing the
-- excluded rows (e.g. undercounting). The base scan, the aggregate scan (Tantivy
-- and DataFusion paths) and the join scan resolve queries against an index at
-- different planning phases, so each gates through `missing_partial_index_predicate`.
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS pig_left, pig_right;
CREATE TABLE pig_left (id serial PRIMARY KEY, category text, active boolean);
INSERT INTO pig_left (category, active)
SELECT (ARRAY['a', 'b', 'c'])[1 + (g % 3)], (g % 2 = 0)
FROM generate_series(1, 900) g;

CREATE TABLE pig_right (id serial PRIMARY KEY, left_id int);
INSERT INTO pig_right (left_id) SELECT g FROM generate_series(1, 900) g;

-- Partial index on the left table: only rows WHERE active. `active` is not indexed.
CREATE INDEX pig_left_idx ON pig_left
USING bm25 (id, category) WITH (key_field = 'id') WHERE active;
CREATE INDEX pig_right_idx ON pig_right
USING bm25 (id, left_id) WITH (key_field = 'id');

SET paradedb.enable_aggregate_custom_scan TO on;

-- Reports whether the plan for `q` uses a given ParadeDB custom scan, without
-- capturing the full plan (whose fallback seq-scan embeds a non-deterministic
-- index oid).
CREATE OR REPLACE FUNCTION plan_uses(q text, needle text) RETURNS boolean AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (COSTS OFF) ' || q LOOP
    IF r."QUERY PLAN" LIKE '%' || needle || '%' THEN RETURN true; END IF;
  END LOOP;
  RETURN false;
END $$ LANGUAGE plpgsql;

-- === Single-table aggregate (Tantivy path) ===
-- Predicate missing: must decline.
SELECT plan_uses(
  $$SELECT count(*) FROM pig_left WHERE category @@@ 'a'$$,
  'ParadeDB Aggregate Scan') AS agg_when_predicate_missing;
-- Predicate present (AND active): used.
SELECT plan_uses(
  $$SELECT count(*) FROM pig_left WHERE category @@@ 'a' AND active$$,
  'ParadeDB Aggregate Scan') AS agg_when_predicate_present;
-- The used aggregate must still count only the matching rows.
SELECT count(*) AS agg_used_count FROM pig_left WHERE category @@@ 'a' AND active;

-- === Aggregate over a join (DataFusion path) ===
-- Predicate missing: must decline.
SELECT plan_uses(
  $$SELECT count(*) FROM pig_left l JOIN pig_right r ON r.left_id = l.id
    WHERE l.category @@@ 'a'$$,
  'ParadeDB Aggregate Scan') AS agg_join_when_predicate_missing;

-- === Pure join scan (no aggregate) ===
-- Predicate missing: must decline.
SELECT plan_uses(
  $$SELECT l.id FROM pig_left l JOIN pig_right r ON r.left_id = l.id
    WHERE l.category @@@ 'a' ORDER BY l.id LIMIT 5$$,
  'ParadeDB Join Scan') AS join_when_predicate_missing;
-- Predicate present (AND active): used.
SELECT plan_uses(
  $$SELECT l.id FROM pig_left l JOIN pig_right r ON r.left_id = l.id
    WHERE l.category @@@ 'a' AND l.active ORDER BY l.id LIMIT 5$$,
  'ParadeDB Join Scan') AS join_when_predicate_present;

DROP FUNCTION plan_uses(text, text);
DROP TABLE pig_left, pig_right;

-- =====================================================================
-- Regression: JoinScan lifts `NOT IN (SELECT col FROM t)` SubPlans into
-- a DataFusion `LeftAnti` join with the `null_aware=true` flag set, so
-- SQL three-valued NULL semantics are preserved.
--
-- SQL semantics: `x NOT IN (... NULL ...)` evaluates to UNKNOWN for any
--   x that doesn't equal a non-NULL element. UNKNOWN in WHERE excludes
--   the row. So a NOT IN against any list containing NULL excludes ALL
--   outer rows.
--
-- Plain DataFusion `LeftAnti`: emit each left row where no
--   `left.x = right.col` evaluates to TRUE. NULL right.col never
--   matches, so it's silently ignored — equivalent to NOT EXISTS, which
--   is *different* from NOT IN.
--
-- Pre-fix behavior: `wrap_with_semi_anti` lifted `is_anti = true`
-- SubPlans into `JoinType::Anti` and the translator lowered them to
-- plain `LeftAnti`. Result: silent wrong row counts (a query that
-- should return 0 rows returned all the non-matching outer rows
-- instead).
--
-- Post-fix behavior: `wrap_with_semi_anti` lifts `is_anti = true`
-- SubPlans to `JoinType::Anti` with `null_aware=true`. The translator
-- routes through `LogicalPlanBuilder::join_detailed_with_options(...,
-- null_aware=true)`. DataFusion's HashJoinExec then tracks
-- `probe_side_has_null` and emits zero rows when any inner key column
-- is NULL, matching SQL three-valued logic.
--
-- DataFusion's null-aware mode is restricted to single-column
-- equi-keys; multi-column `NOT IN`s are still skipped at lift time
-- (Postgres' executor handles those correctly via three-valued logic).
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;
SET work_mem = '1GB';

DROP TABLE IF EXISTS jnns_items CASCADE;
DROP TABLE IF EXISTS jnns_include_set CASCADE;
DROP TABLE IF EXISTS jnns_exclude_set CASCADE;

-- Outer: 100 rows, all with txt='match'.
CREATE TABLE jnns_items (
    id  bigint PRIMARY KEY,
    txt text   NOT NULL
);
INSERT INTO jnns_items SELECT s, 'match' FROM generate_series(1, 100) s;

-- Drives JoinScan engagement (single-table queries don't trigger it).
CREATE TABLE jnns_include_set (
    id   serial PRIMARY KEY,
    val  bigint NOT NULL
);
INSERT INTO jnns_include_set (val) SELECT s FROM generate_series(1, 100) s;

-- The NULL bomb: 11 vals (50..60) plus a single NULL row.
-- Under SQL NOT IN, this NULL poisons every outer row's check.
CREATE TABLE jnns_exclude_set (
    id   serial PRIMARY KEY,
    val  bigint   -- NULLABLE
);
INSERT INTO jnns_exclude_set (val) SELECT s FROM generate_series(50, 60) s;
INSERT INTO jnns_exclude_set (val) VALUES (NULL);

CREATE INDEX jnns_items_idx       ON jnns_items       USING bm25 (id, txt)
  WITH (key_field=id, text_fields='{"txt":{"fast":true}}');
CREATE INDEX jnns_include_set_idx ON jnns_include_set USING bm25 (id, val)
  WITH (key_field=id, numeric_fields='{"val":{"fast":true}}');
CREATE INDEX jnns_exclude_set_idx ON jnns_exclude_set USING bm25 (id, val)
  WITH (key_field=id, numeric_fields='{"val":{"fast":true}}');

ANALYZE jnns_items; ANALYZE jnns_include_set; ANALYZE jnns_exclude_set;

-- =====================================================================
-- Test 1 — Postgres ground truth (all custom scans off).
-- Expected: 0 rows (NULL in jnns_exclude_set poisons NOT IN).
-- =====================================================================
SET paradedb.enable_custom_scan          TO off;
SET paradedb.enable_join_custom_scan     TO off;
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT COUNT(*) AS expected_zero
FROM jnns_items
WHERE id IN     (SELECT val FROM jnns_include_set)
  AND id NOT IN (SELECT val FROM jnns_exclude_set)
  AND txt @@@ 'match';

-- =====================================================================
-- Test 2 — JoinScan enabled. After the fix, should also return 0.
-- Pre-fix this returned 89 (the LeftAnti-as-NOT-EXISTS answer) — wrong.
-- Post-fix the NOT IN SubPlan is lifted to `JoinType::Anti` with
-- `null_aware=true`, so DataFusion correctly returns 0 when the inner
-- (jnns_exclude_set.val) contains a NULL.
-- =====================================================================
SET paradedb.enable_custom_scan      TO on;
SET paradedb.enable_join_custom_scan TO on;

SELECT COUNT(*) AS joinscan_result FROM (
  SELECT jnns_items.id FROM jnns_items
  WHERE id IN     (SELECT val FROM jnns_include_set)
    AND id NOT IN (SELECT val FROM jnns_exclude_set)
    AND txt @@@ 'match'
  ORDER BY jnns_items.id LIMIT 1000
) sub;

-- =====================================================================
-- Test 3 — `NOT EXISTS` rewrite is the safe acceleration path.
-- Different SQL semantics by design (NOT EXISTS doesn't propagate
-- UNKNOWN), so this can return non-zero rows even when NOT IN's answer
-- is zero. Documenting the difference.
-- =====================================================================
SELECT COUNT(*) AS notexists_result FROM (
  SELECT jnns_items.id FROM jnns_items
  WHERE EXISTS     (SELECT 1 FROM jnns_include_set i WHERE i.val = jnns_items.id)
    AND NOT EXISTS (SELECT 1 FROM jnns_exclude_set e WHERE e.val = jnns_items.id)
    AND txt @@@ 'match'
  ORDER BY jnns_items.id LIMIT 1000
) sub;

DROP TABLE jnns_items, jnns_include_set, jnns_exclude_set CASCADE;

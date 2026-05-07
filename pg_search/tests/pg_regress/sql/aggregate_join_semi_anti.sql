-- =====================================================================
-- Aggregate-on-join: accept Semi / Anti joins from EXISTS / NOT EXISTS
-- pull-up, accept Semi joins from IN pull-up, and lift un-pulled-up
-- IN / NOT IN SubPlans into Semi / null-aware Anti joins.
--
-- Regression test for issue #4911. Pre-fix the agg-on-join walker bailed
-- with `unexpected node type T_FromExpr in join tree` (the parse-tree
-- shape PG produces post-pull-up) or with `aggregate-on-join does not
-- support Anti JOIN` (the join-type allow-list rejected Semi/Anti even
-- when the walker got past T_FromExpr).
--
-- Coverage in this file:
--   Test 1: IN (SELECT ...)  ->  pushed down via JoinExpr {SEMI}.
--   Test 2: EXISTS / NOT EXISTS  ->  pushed down via JoinExpr {SEMI/ANTI}.
--   Test 3: single-column NOT IN un-pulled-up  ->  pushed down via the
--           null-aware Anti lift in `wrap_with_semi_anti`.
--   Test 4: result parity for the Test 3 query with the customscan off.
--   Test 5: multi-column NOT IN un-pulled-up  ->  declined cleanly
--           (DataFusion null-aware mode requires a single-column
--           equi-key). PG handles the SubPlan via
--           `nodeSubplan.c::ExecHashSubPlan`.
--   Test 6: single-column NOT IN with a NULL-bearing inner  ->  pushed
--           down with `null_aware=true`; HashJoinExec emits zero rows
--           because the probe side has a NULL, matching SQL three-valued
--           logic. Includes a sanity check (delete the NULL, query
--           returns non-zero rows) to guard against trivially passing
--           with zero-rows-for-the-wrong-reason.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test data
-- =====================================================================
CREATE TABLE asa_cccf (
    contact_id   bigint PRIMARY KEY,
    job_title    text
);

CREATE TABLE asa_contact_list (
    id      serial PRIMARY KEY,
    list_id text NOT NULL,
    ldf_id  bigint NOT NULL
);

INSERT INTO asa_cccf
SELECT s,
       CASE WHEN s % 5 = 0 THEN 'Senior Programmer' ELSE 'Other' END
FROM generate_series(1, 100) s;

INSERT INTO asa_contact_list (list_id, ldf_id)
SELECT 'list-A', s FROM generate_series(1, 50) s;
INSERT INTO asa_contact_list (list_id, ldf_id)
SELECT 'list-B', s FROM generate_series(40, 60) s;

CREATE INDEX asa_cccf_idx ON asa_cccf
USING bm25 (contact_id, job_title)
WITH (
    key_field='contact_id',
    text_fields='{"job_title":{"fast":true}}'
);

CREATE INDEX asa_contact_list_idx ON asa_contact_list
USING bm25 (id, (list_id::pdb.literal), ldf_id)
WITH (key_field='id');

ANALYZE asa_cccf;
ANALYZE asa_contact_list;

-- =====================================================================
-- Test 1: GROUP BY aggregate over IN - pulls up to JoinExpr {SEMI},
--         must use Aggregate Scan
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT job_title, COUNT(*) AS doc_count
FROM asa_cccf
WHERE contact_id IN (SELECT ldf_id FROM asa_contact_list WHERE list_id @@@ 'list-A')
  AND job_title @@@ 'Senior'
GROUP BY job_title
ORDER BY doc_count DESC, job_title;

SELECT job_title, COUNT(*) AS doc_count
FROM asa_cccf
WHERE contact_id IN (SELECT ldf_id FROM asa_contact_list WHERE list_id @@@ 'list-A')
  AND job_title @@@ 'Senior'
GROUP BY job_title
ORDER BY doc_count DESC, job_title;

-- =====================================================================
-- Test 2: GROUP BY aggregate over NOT EXISTS - pulls up to JoinExpr
--         {ANTI}, must use Aggregate Scan. This is the recommended
--         rewrite for `NOT IN` queries.
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT job_title, COUNT(*) AS doc_count
FROM asa_cccf c
WHERE EXISTS     (SELECT 1 FROM asa_contact_list cl WHERE cl.ldf_id = c.contact_id AND cl.list_id @@@ 'list-A')
  AND NOT EXISTS (SELECT 1 FROM asa_contact_list cl WHERE cl.ldf_id = c.contact_id AND cl.list_id @@@ 'list-B')
  AND c.job_title @@@ 'Senior'
GROUP BY job_title
ORDER BY doc_count DESC, job_title;

SELECT job_title, COUNT(*) AS doc_count
FROM asa_cccf c
WHERE EXISTS     (SELECT 1 FROM asa_contact_list cl WHERE cl.ldf_id = c.contact_id AND cl.list_id @@@ 'list-A')
  AND NOT EXISTS (SELECT 1 FROM asa_contact_list cl WHERE cl.ldf_id = c.contact_id AND cl.list_id @@@ 'list-B')
  AND c.job_title @@@ 'Senior'
GROUP BY job_title
ORDER BY doc_count DESC, job_title;

-- =====================================================================
-- Test 3: single-column NOT IN that Postgres does NOT pull up to ANTI -
--         pushes down via the null-aware Anti lift in `wrap_with_semi_anti`
--         (single-column equi-key is the supported case for null_aware=true).
--         The inner column `ldf_id` here is NOT NULL, so null-aware mode is
--         exercised but doesn't fire its zero-rows fast path - the result
--         is the same as plain LeftAnti would produce. Test 6 below exercises
--         the actual NULL-bomb behavior. This case must use Aggregate Scan
--         and return rows that match `enable_aggregate_custom_scan = off`.
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT job_title, COUNT(*) AS doc_count
FROM asa_cccf
WHERE contact_id IN     (SELECT ldf_id FROM asa_contact_list WHERE list_id @@@ 'list-A')
  AND contact_id NOT IN (SELECT ldf_id FROM asa_contact_list WHERE list_id @@@ 'list-B')
  AND job_title @@@ 'Senior'
GROUP BY job_title
ORDER BY doc_count DESC, job_title;

SELECT job_title, COUNT(*) AS doc_count
FROM asa_cccf
WHERE contact_id IN     (SELECT ldf_id FROM asa_contact_list WHERE list_id @@@ 'list-A')
  AND contact_id NOT IN (SELECT ldf_id FROM asa_contact_list WHERE list_id @@@ 'list-B')
  AND job_title @@@ 'Senior'
GROUP BY job_title
ORDER BY doc_count DESC, job_title;

-- =====================================================================
-- Test 4: result parity - the NOT IN form with the customscan disabled
--         must return the same rows as Test 3 above (Postgres-only
--         execution of the same logical query).
-- =====================================================================
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT job_title, COUNT(*) AS doc_count
FROM asa_cccf
WHERE contact_id IN     (SELECT ldf_id FROM asa_contact_list WHERE list_id @@@ 'list-A')
  AND contact_id NOT IN (SELECT ldf_id FROM asa_contact_list WHERE list_id @@@ 'list-B')
  AND job_title @@@ 'Senior'
GROUP BY job_title
ORDER BY doc_count DESC, job_title;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 5: multi-column NOT IN inside an agg-on-join - wrap_with_semi_anti
--         cannot lift the SubPlan (DataFusion null-aware mode requires a
--         single-column equi-key) and the agg-on-join caller MUST detect
--         the unabsorbed SubPlan and decline pushdown so Postgres handles
--         it via `nodeSubplan.c`. Without the absorption check the agg
--         scan would silently drop the predicate and report wrong counts.
--
--         Shape: `IN (...)` (PG pulls up to JOIN_SEMI, forcing the
--         agg-on-join path) + `(a, b) NOT IN (...)` (stays attached to
--         baserestrictinfo as a SubPlan because of the multi-column key).
-- =====================================================================
CREATE TABLE asa_pair_outer (
    id    bigint PRIMARY KEY,
    a     int NOT NULL,
    b     int NOT NULL,
    label text NOT NULL
);
CREATE TABLE asa_pair_inner (
    pid serial PRIMARY KEY,
    x   int NOT NULL,
    y   int NOT NULL
);
CREATE TABLE asa_pair_include (
    id bigint NOT NULL
);

INSERT INTO asa_pair_outer
SELECT s, s, s,
       CASE WHEN s % 5 = 0 THEN 'Senior Programmer' ELSE 'Other' END
FROM generate_series(1, 20) s;

INSERT INTO asa_pair_inner (x, y) VALUES (5, 5), (10, 10);

INSERT INTO asa_pair_include
SELECT s FROM generate_series(1, 15) s;

CREATE INDEX asa_pair_outer_idx ON asa_pair_outer
USING bm25 (id, label)
WITH (key_field='id', text_fields='{"label":{"fast":true}}');

CREATE INDEX asa_pair_include_idx ON asa_pair_include
USING bm25 (id) WITH (key_field='id');

CREATE INDEX asa_pair_inner_idx ON asa_pair_inner
USING bm25 (pid, x, y) WITH (key_field='pid');

ANALYZE asa_pair_outer;
ANALYZE asa_pair_inner;
ANALYZE asa_pair_include;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT label, COUNT(*) AS doc_count
FROM asa_pair_outer
WHERE id IN (SELECT id FROM asa_pair_include)
  AND (a, b) NOT IN (SELECT x, y FROM asa_pair_inner)
  AND label @@@ 'Senior'
GROUP BY label
ORDER BY doc_count DESC, label;

SELECT label, COUNT(*) AS doc_count
FROM asa_pair_outer
WHERE id IN (SELECT id FROM asa_pair_include)
  AND (a, b) NOT IN (SELECT x, y FROM asa_pair_inner)
  AND label @@@ 'Senior'
GROUP BY label
ORDER BY doc_count DESC, label;

-- Parity vs PG plan: same query with custom scan OFF must match.
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT label, COUNT(*) AS doc_count
FROM asa_pair_outer
WHERE id IN (SELECT id FROM asa_pair_include)
  AND (a, b) NOT IN (SELECT x, y FROM asa_pair_inner)
  AND label @@@ 'Senior'
GROUP BY label
ORDER BY doc_count DESC, label;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 6: NOT IN with a NULL-bearing inner - the load-bearing case for
--         `null_aware=true` on the lifted Anti join. Under SQL three-
--         valued logic, `x NOT IN (... NULL ...)` is UNKNOWN for every
--         outer row, so the WHERE excludes ALL rows. Plain DataFusion
--         `LeftAnti` ignores inner NULLs (== NOT EXISTS) and would return
--         the wrong answer; `null_aware=true` makes `HashJoinExec` emit
--         zero rows whenever the probe (inner) side has any NULL.
--
--         Shape: `IN (...)` (PG pulls up to JOIN_SEMI, forcing the
--         agg-on-join path) + single-column `NOT IN (...)` over a
--         NULL-bearing inner (un-pulled-up SubPlan, lifted by
--         `wrap_with_semi_anti` with `null_aware=true`).
--
--         Asserts:
--           1. EXPLAIN: pushes down to `Custom Scan (ParadeDB Aggregate
--              Scan)` (Backend: DataFusion). A regression that disabled
--              the null-aware lift would force a decline and this would
--              show a Postgres plan.
--           2. Result count = 0 (matches PG three-valued semantics).
--           3. Parity with `enable_aggregate_custom_scan = off`.
--           4. Sanity: removing the NULL inner row makes the same query
--              return non-zero rows - guards against a silently broken
--              setup where the NULL is missing and Test 6 "passes" with
--              zero rows for the wrong reason.
--
--         Not asserted here: the underlying DataFusion physical plan
--         shape (HashJoinExec with `null_equality=NullEqualsNothing`).
--         AggregateScan's EXPLAIN doesn't render the physical plan
--         tree the way JoinScan's does - surfacing it in EXPLAIN
--         (or asserting it via a Rust test on `build_physical_plan`)
--         would catch a future regression where HashJoinExec degrades
--         to NestedLoopJoin without changing the result. Tracked as a
--         follow-up.
-- =====================================================================
CREATE TABLE asa_excl_outer (
    id    bigint PRIMARY KEY,
    label text NOT NULL
);
CREATE TABLE asa_excl_include (
    id bigint NOT NULL
);
CREATE TABLE asa_excl_inner (
    iid bigint PRIMARY KEY,
    eid bigint            -- nullable on purpose, will hold a NULL row
);

INSERT INTO asa_excl_outer
SELECT s,
       CASE WHEN s % 5 = 0 THEN 'Senior Programmer' ELSE 'Other' END
FROM generate_series(1, 20) s;

INSERT INTO asa_excl_include
SELECT s FROM generate_series(1, 20) s;

-- Inner has both a real value AND a NULL. The NULL is what poisons NOT IN.
INSERT INTO asa_excl_inner (iid, eid) VALUES (1, 7), (2, NULL);

CREATE INDEX asa_excl_outer_idx ON asa_excl_outer
USING bm25 (id, label)
WITH (key_field='id', text_fields='{"label":{"fast":true}}');

CREATE INDEX asa_excl_include_idx ON asa_excl_include
USING bm25 (id) WITH (key_field='id');

CREATE INDEX asa_excl_inner_idx ON asa_excl_inner
USING bm25 (iid, eid) WITH (key_field='iid');

ANALYZE asa_excl_outer;
ANALYZE asa_excl_include;
ANALYZE asa_excl_inner;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT label, COUNT(*) AS doc_count
FROM asa_excl_outer
WHERE id IN     (SELECT id FROM asa_excl_include)
  AND id NOT IN (SELECT eid FROM asa_excl_inner)
  AND label @@@ 'Senior'
GROUP BY label
ORDER BY doc_count DESC, label;

SELECT label, COUNT(*) AS doc_count
FROM asa_excl_outer
WHERE id IN     (SELECT id FROM asa_excl_include)
  AND id NOT IN (SELECT eid FROM asa_excl_inner)
  AND label @@@ 'Senior'
GROUP BY label
ORDER BY doc_count DESC, label;

-- Result parity with Postgres-only execution. Both should report zero
-- rows because of the NULL in `asa_excl_inner.eid`.
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT label, COUNT(*) AS doc_count
FROM asa_excl_outer
WHERE id IN     (SELECT id FROM asa_excl_include)
  AND id NOT IN (SELECT eid FROM asa_excl_inner)
  AND label @@@ 'Senior'
GROUP BY label
ORDER BY doc_count DESC, label;

SET paradedb.enable_aggregate_custom_scan TO on;

-- Sanity check: removing the NULL inner row makes the same query return
-- the non-zero answer (PG-only ground truth). This guards against a
-- silently broken setup where the NULL is missing and Test 6 "passes"
-- with zero rows for the wrong reason.
DELETE FROM asa_excl_inner WHERE eid IS NULL;
ANALYZE asa_excl_inner;

SELECT label, COUNT(*) AS doc_count
FROM asa_excl_outer
WHERE id IN     (SELECT id FROM asa_excl_include)
  AND id NOT IN (SELECT eid FROM asa_excl_inner)
  AND label @@@ 'Senior'
GROUP BY label
ORDER BY doc_count DESC, label;

-- =====================================================================
-- Cleanup
-- =====================================================================
DROP TABLE asa_excl_outer;
DROP TABLE asa_excl_include;
DROP TABLE asa_excl_inner;
DROP TABLE asa_pair_outer;
DROP TABLE asa_pair_inner;
DROP TABLE asa_pair_include;
DROP TABLE asa_contact_list;
DROP TABLE asa_cccf;

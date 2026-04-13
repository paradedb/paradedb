-- Tests for LIMIT pushdown suppression in BaseScan when non-pushable
-- post-filters exist (SubPlan, volatile functions).
--
-- The bug: BaseScan absorbs the query-level LIMIT as a hard output cap
-- inside TopK, but SubPlan predicates (e.g. IN (SELECT ...)) are evaluated
-- by Postgres AFTER the scan. Rows discarded by the post-filter come from
-- an already-capped output, producing fewer results than correct.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = off;

-- ============================================================
-- Shared setup
-- ============================================================

DROP TABLE IF EXISTS lp_categories CASCADE;
DROP TABLE IF EXISTS lp_items CASCADE;
DROP TABLE IF EXISTS lp_items_part CASCADE;
DROP TABLE IF EXISTS lp_left_table CASCADE;
DROP TABLE IF EXISTS lp_active_statuses CASCADE;
DROP TABLE IF EXISTS lp_allowed_tenants CASCADE;

CREATE TABLE lp_categories (
    id BIGINT PRIMARY KEY,
    name TEXT NOT NULL
) WITH (autovacuum_enabled = false);

CREATE TABLE lp_items (
    id BIGINT PRIMARY KEY,
    category_id BIGINT,
    tenant_id BIGINT,
    status TEXT,
    fk BIGINT,
    description TEXT NOT NULL
) WITH (autovacuum_enabled = false);

CREATE TABLE lp_left_table (
    id BIGINT PRIMARY KEY,
    status TEXT
) WITH (autovacuum_enabled = false);

CREATE TABLE lp_active_statuses (
    s TEXT PRIMARY KEY
) WITH (autovacuum_enabled = false);

CREATE TABLE lp_allowed_tenants (
    tenant_id BIGINT,
    user_id TEXT
) WITH (autovacuum_enabled = false);

-- Populate categories: only 'rare_category' will match the SubPlan filter.
INSERT INTO lp_categories VALUES (1, 'rare_category');
INSERT INTO lp_categories VALUES (2, 'common_category');

-- Populate items: 1000 rows all match 'searchable'. Rows 151-1000 repeat
-- the term 5x so they score much higher than rows 1-150. TopK by score
-- DESC would pick rows 151-1000 first — those all have category_id=999
-- and fail the SubPlan filter. With the bug: 0 rows. With the fix: 50.
INSERT INTO lp_items
SELECT i,
       CASE WHEN i <= 150 THEN NULL ELSE 999 END,
       1, 'active', i,
       CASE WHEN i <= 150 THEN 'searchable'
            ELSE 'searchable searchable searchable searchable searchable'
       END
FROM generate_series(1, 1000) i;

-- Make the BM25 index
CREATE INDEX lp_items_bm25 ON lp_items
USING bm25 (id, category_id, tenant_id, status, fk, description)
WITH (key_field = 'id');

-- Active statuses for lateral/partitioned tests
INSERT INTO lp_active_statuses VALUES ('active');

-- Left table for LATERAL test
INSERT INTO lp_left_table
SELECT i, 'active' FROM generate_series(1, 100) i;

-- Allowed tenants for RLS test (must match the role name exactly)
INSERT INTO lp_allowed_tenants VALUES (1, 'lp_restricted_user');

ANALYZE;

-- ============================================================
-- Test 1: Core reproducer — SubPlan post-filter with LIMIT
-- ============================================================
-- The SubPlan (IN (SELECT ...)) cannot be pushed to Tantivy.
-- With the bug, TopK caps output before the SubPlan filter runs,
-- producing ~0 rows instead of 50.
-- Expected: 50 rows. EXPLAIN should show NormalScanExecState.

SELECT count(*) FROM (
    SELECT id FROM lp_items
    WHERE description @@@ 'searchable'
      AND (category_id IS NULL
           OR category_id IN (
               SELECT id FROM lp_categories
               WHERE name = 'rare_category'))
    ORDER BY paradedb.score(id) DESC
    LIMIT 50
) sub;

-- Verify EXPLAIN shows NormalScanExecState, not TopKScanExecState
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM lp_items
WHERE description @@@ 'searchable'
  AND (category_id IS NULL
       OR category_id IN (
           SELECT id FROM lp_categories
           WHERE name = 'rare_category'))
ORDER BY paradedb.score(id) DESC
LIMIT 50;

-- ============================================================
-- Test 2: No-LIMIT regression — same predicate without LIMIT
-- ============================================================
-- All qualifying rows should be returned. No LIMIT means no TopK issue.

SELECT count(*) FROM (
    SELECT id FROM lp_items
    WHERE description @@@ 'searchable'
      AND (category_id IS NULL
           OR category_id IN (
               SELECT id FROM lp_categories
               WHERE name = 'rare_category'))
) sub;

-- ============================================================
-- Test 3: Fully-pushable + LIMIT regression
-- ============================================================
-- All predicates pushed to Tantivy. TopK SHOULD still be used.

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM lp_items
WHERE description @@@ 'searchable' LIMIT 100;

-- ============================================================
-- Test 4: Parameterized limit via prepared statement
-- ============================================================

PREPARE lp_q(int) AS
SELECT id FROM lp_items
WHERE description @@@ 'searchable'
  AND (category_id IS NULL
       OR category_id IN (
           SELECT id FROM lp_categories WHERE name = 'rare_category'))
ORDER BY paradedb.score(id) DESC
LIMIT $1;

CREATE TEMP TABLE lp_q_result AS EXECUTE lp_q(100);
SELECT count(*) FROM lp_q_result;
DROP TABLE lp_q_result;

DEALLOCATE lp_q;

-- ============================================================
-- Test 5: Partition with SubPlan post-filter
-- ============================================================
-- BM25 indexes live on individual partitions. Query a partition
-- directly to verify that has_non_pushable_predicates() catches
-- the SubPlan even though rel_is_single_or_partitioned passes.

CREATE TABLE lp_items_part (
    id BIGINT NOT NULL,
    status TEXT,
    description TEXT NOT NULL
) PARTITION BY RANGE (id);

CREATE TABLE lp_items_part_1 PARTITION OF lp_items_part FOR VALUES FROM (1) TO (501);
CREATE TABLE lp_items_part_2 PARTITION OF lp_items_part FOR VALUES FROM (501) TO (1001);

INSERT INTO lp_items_part
SELECT i,
       CASE WHEN i <= 500 THEN 'active' ELSE 'inactive' END,
       'partitioned item'
FROM generate_series(1, 1000) i;

CREATE INDEX lp_items_part_1_bm25 ON lp_items_part_1
USING bm25 (id, status, description) WITH (key_field = 'id');
CREATE INDEX lp_items_part_2_bm25 ON lp_items_part_2
USING bm25 (id, status, description) WITH (key_field = 'id');

ANALYZE;

-- Query partition directly (not the parent table)
SELECT count(*) FROM (
    SELECT id FROM lp_items_part_1
    WHERE description @@@ 'partitioned'
      AND (status IS NULL
           OR status IN (SELECT s FROM lp_active_statuses))
    LIMIT 100
) sub;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM lp_items_part_1
WHERE description @@@ 'partitioned'
  AND (status IS NULL
       OR status IN (SELECT s FROM lp_active_statuses))
LIMIT 100;

-- ============================================================
-- Test 6: LEFT JOIN LATERAL with SubPlan on the outer table
-- ============================================================
-- The SubPlan is on the outer table (lp_left_table), so limit should
-- be suppressed for safety.

SELECT count(*) FROM (
    SELECT l.id FROM lp_left_table l
    LEFT JOIN LATERAL (
        SELECT * FROM lp_items i
        WHERE i.fk = l.id AND i.description @@@ 'searchable'
    ) sub ON true
    WHERE l.status IN (SELECT s FROM lp_active_statuses)
    LIMIT 50
) result;

-- Verify BaseScan does not use TopK for the LATERAL subquery
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT l.id FROM lp_left_table l
LEFT JOIN LATERAL (
    SELECT * FROM lp_items i
    WHERE i.fk = l.id AND i.description @@@ 'searchable'
) sub ON true
WHERE l.status IN (SELECT s FROM lp_active_statuses)
LIMIT 50;

-- ============================================================
-- Test 7: RLS-injected SubPlan (subquery-based policy)
-- ============================================================
-- Row-level security adds a SubPlan to baserestrictinfo that can't
-- be pushed to Tantivy.

-- Create a role for RLS testing
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'lp_restricted_user') THEN
        CREATE ROLE lp_restricted_user LOGIN;
    END IF;
END
$$;

GRANT SELECT ON lp_items TO lp_restricted_user;
GRANT SELECT ON lp_categories TO lp_restricted_user;
GRANT SELECT ON lp_allowed_tenants TO lp_restricted_user;

ALTER TABLE lp_items ENABLE ROW LEVEL SECURITY;
CREATE POLICY lp_tenant_isolation ON lp_items
    USING (tenant_id IN (
        SELECT tenant_id FROM lp_allowed_tenants
        WHERE user_id = current_user
    ));

SET ROLE lp_restricted_user;

-- With the RLS policy active, TopK should be suppressed
SELECT count(*) FROM (
    SELECT id FROM lp_items
    WHERE description @@@ 'searchable'
    ORDER BY paradedb.score(id) DESC
    LIMIT 100
) sub;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM lp_items
WHERE description @@@ 'searchable'
ORDER BY paradedb.score(id) DESC
LIMIT 100;

RESET ROLE;

-- ============================================================
-- Cleanup
-- ============================================================

ALTER TABLE lp_items DISABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS lp_tenant_isolation ON lp_items;

DROP TABLE IF EXISTS lp_items_part CASCADE;
DROP TABLE IF EXISTS lp_left_table CASCADE;
DROP TABLE IF EXISTS lp_active_statuses CASCADE;
DROP TABLE IF EXISTS lp_allowed_tenants CASCADE;
DROP TABLE IF EXISTS lp_categories CASCADE;
DROP TABLE IF EXISTS lp_items CASCADE;
DROP ROLE IF EXISTS lp_restricted_user;

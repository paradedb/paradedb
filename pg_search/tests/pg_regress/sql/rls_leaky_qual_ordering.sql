-- RLS leaky-qual ordering under ParadeDB filter pushdown.
--
-- PostgreSQL never evaluates a non-leakproof predicate on a row before the row passes the
-- relation's RLS policy quals (restriction_is_securely_promotable). The custom scans must
-- not break that by pushing such a predicate into the scan.
--
-- Probe: `secret::int` is non-leakproof (a failed cast reports the value). Org-1 secrets
-- are integers, org-2 secrets are not, so any ORG2-* value in an error is a leak.

CREATE EXTENSION IF NOT EXISTS pg_search;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        CREATE ROLE authenticated;
    END IF;
END
$$;

DO $$
DECLARE
    v_session_user name := session_user;
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_auth_members m
        JOIN pg_roles r_member ON r_member.oid = m.member
        JOIN pg_roles r_role   ON r_role.oid   = m.roleid
        WHERE r_member.rolname = v_session_user
          AND r_role.rolname = 'authenticated'
    ) THEN
        EXECUTE format('GRANT authenticated TO %I', v_session_user);
    END IF;
END
$$;

DROP TABLE IF EXISTS leaky_rls_docs CASCADE;
DROP TABLE IF EXISTS memberships CASCADE;

-- Which role may see which org. Drives a subquery-based RLS policy.
CREATE TABLE memberships (role_name text NOT NULL, org_id int NOT NULL);
INSERT INTO memberships (role_name, org_id) VALUES ('authenticated', 1);
GRANT SELECT ON memberships TO authenticated;

CREATE TABLE leaky_rls_docs (
    id     bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    org_id int  NOT NULL,
    secret text NOT NULL,
    body   text NOT NULL
);

-- `body` matches the search term in BOTH orgs. Org-1 secrets are castable to int;
-- org-2 secrets are not, so casting an org-2 row raises an error embedding the value.
INSERT INTO leaky_rls_docs (org_id, secret, body) VALUES
    (1, '111',                   'sheriff department report'),
    (1, '222',                   'sheriff patrol log'),
    (2, 'ORG2-TOPSECRET-uvwxyz', 'sheriff incident summary'),
    (2, 'ORG2-CLASSIFIED-98765', 'sheriff dispatch record');

CREATE INDEX leaky_rls_bm25 ON leaky_rls_docs
    USING paradedb (id, body, secret, org_id);

GRANT SELECT ON leaky_rls_docs TO authenticated;
ALTER TABLE leaky_rls_docs ENABLE ROW LEVEL SECURITY;

-- Subquery RLS policy -> becomes a SubPlan that stays above the scan.
CREATE POLICY org_isolation ON leaky_rls_docs FOR SELECT
    USING (org_id IN (SELECT m.org_id FROM memberships m WHERE m.role_name = current_user));

--------------------------------------------------------------------------------
-- CASE A: ParadeDB filter pushdown ON (default). @@@ present.
-- Safe = rows 1,2 and NO error. A cast error naming an ORG2-* value is an RLS leak.
--------------------------------------------------------------------------------
BEGIN;
SET LOCAL ROLE authenticated;
SELECT id, org_id
FROM leaky_rls_docs
WHERE body @@@ 'sheriff'
  AND secret::int > 0
ORDER BY id;
COMMIT;

--------------------------------------------------------------------------------
-- CASE B (control): identical query, filter pushdown OFF. The cast stays a
-- PostgreSQL post-filter, correctly ordered after RLS, so it must be safe.
--------------------------------------------------------------------------------
BEGIN;
SET LOCAL paradedb.enable_filter_pushdown = off;
SET LOCAL ROLE authenticated;
SELECT id, org_id
FROM leaky_rls_docs
WHERE body @@@ 'sheriff'
  AND secret::int > 0
ORDER BY id;
COMMIT;

--------------------------------------------------------------------------------
-- CASE C (reference): stock PostgreSQL, no @@@, custom scan disabled. Safe baseline.
--------------------------------------------------------------------------------
BEGIN;
SET LOCAL paradedb.enable_custom_scan = off;
SET LOCAL ROLE authenticated;
SELECT id, org_id
FROM leaky_rls_docs
WHERE secret::int > 0
ORDER BY id;
COMMIT;

--------------------------------------------------------------------------------
-- CASE D: the leaky predicate hidden inside an OR with an `@@@`. The clause can
-- neither be lowered into the scan nor deferred above it, so the custom scan must
-- step aside. Safe = rows 1,2 and NO error.
--------------------------------------------------------------------------------
BEGIN;
SET LOCAL ROLE authenticated;
SELECT id, org_id
FROM leaky_rls_docs
WHERE body @@@ 'sheriff'
  AND (body @@@ 'zzznomatch' OR secret::int > 0)
ORDER BY id;
COMMIT;

--------------------------------------------------------------------------------
-- CASE E: yes/no oracle. Division by zero fires only if the predicate is
-- evaluated on a hidden ORG2-* row. Safe = rows 1,2 and NO error.
--------------------------------------------------------------------------------
BEGIN;
SET LOCAL ROLE authenticated;
SELECT id, org_id
FROM leaky_rls_docs
WHERE body @@@ 'sheriff'
  AND (CASE WHEN secret LIKE 'ORG2-%' THEN 1 ELSE 0 END)
      / (CASE WHEN secret LIKE 'ORG2-%' THEN 0 ELSE 1 END) = 0
ORDER BY id;
COMMIT;

DROP POLICY org_isolation ON leaky_rls_docs;
DROP TABLE leaky_rls_docs CASCADE;
DROP TABLE memberships CASCADE;

--------------------------------------------------------------------------------
-- CASE F: a deferred predicate must not be combined with LIMIT pushdown. TopK
-- would count rows that the deferred plan.qual later rejects and stop early.
-- Rows 1..100 fail the predicate, so the correct answer is 101..105.
--------------------------------------------------------------------------------
CREATE TABLE leaky_rls_topk (id int PRIMARY KEY, body text NOT NULL, tag text NOT NULL);
INSERT INTO leaky_rls_topk
SELECT i, 'sheriff', CASE WHEN i <= 100 THEN 'no' ELSE 'yes' END
FROM generate_series(1, 200) i;
CREATE INDEX leaky_rls_topk_bm25 ON leaky_rls_topk USING paradedb (id, body, tag);
GRANT SELECT ON leaky_rls_topk TO authenticated;
ALTER TABLE leaky_rls_topk ENABLE ROW LEVEL SECURITY;
CREATE POLICY topk_visible ON leaky_rls_topk FOR SELECT USING (id >= 0);

BEGIN;
SET LOCAL ROLE authenticated;
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id FROM leaky_rls_topk
WHERE body @@@ 'sheriff' AND tag LIKE 'yes%'
ORDER BY id LIMIT 5;
SELECT id FROM leaky_rls_topk
WHERE body @@@ 'sheriff' AND tag LIKE 'yes%'
ORDER BY id LIMIT 5;
COMMIT;

DROP POLICY topk_visible ON leaky_rls_topk;
DROP TABLE leaky_rls_topk CASCADE;

--------------------------------------------------------------------------------
-- Aggregate Scan and Join Scan have no PostgreSQL filter step above the scan,
-- so they must step aside (leaving the query to the Base Scan) instead of
-- lowering the leaky predicate next to the RLS policy. A simple policy is used
-- here because it is pushed into the Tantivy query as well.
--------------------------------------------------------------------------------
CREATE TABLE leaky_rls_agg (id int PRIMARY KEY, org_id int NOT NULL, secret text NOT NULL, body text NOT NULL);
INSERT INTO leaky_rls_agg VALUES
    (1, 1, '111',                   'sheriff department report'),
    (2, 1, '222',                   'sheriff patrol log'),
    (3, 2, 'ORG2-TOPSECRET-uvwxyz', 'sheriff incident summary'),
    (4, 2, 'ORG2-CLASSIFIED-98765', 'sheriff dispatch record');
CREATE INDEX leaky_rls_agg_bm25 ON leaky_rls_agg USING paradedb (id, body, secret, org_id);
CREATE TABLE leaky_rls_tags (id int PRIMARY KEY, doc_id int NOT NULL, label text NOT NULL);
INSERT INTO leaky_rls_tags SELECT i, ((i - 1) % 4) + 1, 'sheriff tag' FROM generate_series(1, 8) i;
CREATE INDEX leaky_rls_tags_bm25 ON leaky_rls_tags USING paradedb (id, doc_id, label);
GRANT SELECT ON leaky_rls_agg, leaky_rls_tags TO authenticated;
ALTER TABLE leaky_rls_agg ENABLE ROW LEVEL SECURITY;
CREATE POLICY agg_org ON leaky_rls_agg FOR SELECT USING (org_id = 1);

-- CASE G: Aggregate Scan. Expect a Base Scan with the cast in its Filter.
BEGIN;
SET LOCAL ROLE authenticated;
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT count(*) FROM leaky_rls_agg WHERE body @@@ 'sheriff' AND secret::int > 0;
SELECT count(*) FROM leaky_rls_agg WHERE body @@@ 'sheriff' AND secret::int > 0;
COMMIT;

-- CASE H: aggregate FILTER (WHERE ...) is evaluated after RLS by PostgreSQL.
BEGIN;
SET LOCAL ROLE authenticated;
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT count(*) FILTER (WHERE secret::int > 0) FROM leaky_rls_agg WHERE body @@@ 'sheriff';
SELECT count(*) FILTER (WHERE secret::int > 0) FROM leaky_rls_agg WHERE body @@@ 'sheriff';
COMMIT;

-- CASE I: Join Scan. Expect no Join Scan; safe = (1,1),(1,5),(2,2),(2,6).
BEGIN;
SET LOCAL ROLE authenticated;
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT d.id, t.id FROM leaky_rls_agg d JOIN leaky_rls_tags t ON t.doc_id = d.id
WHERE d.body @@@ 'sheriff' AND t.label @@@ 'sheriff' AND d.secret::int > 0
ORDER BY d.id, t.id LIMIT 10;
SELECT d.id, t.id FROM leaky_rls_agg d JOIN leaky_rls_tags t ON t.doc_id = d.id
WHERE d.body @@@ 'sheriff' AND t.label @@@ 'sheriff' AND d.secret::int > 0
ORDER BY d.id, t.id LIMIT 10;
COMMIT;

-- CASE J: OR-subquery policy (`is_public OR org_id IN (SELECT ...)`), which the
-- Join Scan lifts into a join above the scan. Safe = rows 1,2 and NO error.
DROP POLICY agg_org ON leaky_rls_agg;
CREATE TABLE leaky_rls_members (role_name text NOT NULL, org_id int NOT NULL);
INSERT INTO leaky_rls_members VALUES ('authenticated', 1);
CREATE INDEX leaky_rls_members_bm25 ON leaky_rls_members USING paradedb (org_id, role_name);
GRANT SELECT ON leaky_rls_members TO authenticated;
ALTER TABLE leaky_rls_agg ADD COLUMN is_public boolean NOT NULL DEFAULT false;
CREATE POLICY agg_org_or ON leaky_rls_agg FOR SELECT
    USING (is_public OR org_id IN (SELECT m.org_id FROM leaky_rls_members m WHERE m.role_name = current_user));
BEGIN;
SET LOCAL ROLE authenticated;
SELECT id FROM leaky_rls_agg WHERE body @@@ 'sheriff' AND secret::int > 0 ORDER BY id;
SELECT id FROM leaky_rls_agg WHERE body @@@ 'sheriff' AND secret::int > 0 ORDER BY id LIMIT 10;
COMMIT;

DROP POLICY agg_org_or ON leaky_rls_agg;
DROP TABLE leaky_rls_agg, leaky_rls_tags, leaky_rls_members CASCADE;

--------------------------------------------------------------------------------
-- CASE K: stacked policies. PostgreSQL gives each policy its own security level
-- (restrictive ones first, then the permissive OR) and evaluates them in that
-- order before the caller's predicates. Here:
--   level 0: restrictive SubPlan policy that admits every org (stays in plan.qual)
--   level 1: permissive non-leakproof function admitting org 1 only (deferred)
--   level 2: the caller's correlated SubPlan with the leaky cast
-- plan.qual must keep that order, so the cast never sees an org-2 row.
-- Safe = rows 1,2 and NO error.
--------------------------------------------------------------------------------
CREATE TABLE leaky_rls_stack (id int PRIMARY KEY, org_id int NOT NULL, secret text NOT NULL, body text NOT NULL);
INSERT INTO leaky_rls_stack VALUES
    (1, 1, '111',                   'sheriff department report'),
    (2, 1, '222',                   'sheriff patrol log'),
    (3, 2, 'ORG2-TOPSECRET-uvwxyz', 'sheriff incident summary'),
    (4, 2, 'ORG2-CLASSIFIED-98765', 'sheriff dispatch record');
CREATE INDEX leaky_rls_stack_bm25 ON leaky_rls_stack USING paradedb (id, body, secret, org_id);
CREATE TABLE leaky_rls_stack_orgs (org_id int NOT NULL);
INSERT INTO leaky_rls_stack_orgs VALUES (1), (2);
GRANT SELECT ON leaky_rls_stack, leaky_rls_stack_orgs TO authenticated;
-- plpgsql so it is not inlined into a (leakproof) `org_id = 1`.
CREATE FUNCTION leaky_rls_stack_org_check(int) RETURNS boolean
    LANGUAGE plpgsql STABLE AS $$ BEGIN RETURN $1 = 1; END $$;
ALTER TABLE leaky_rls_stack ENABLE ROW LEVEL SECURITY;
CREATE POLICY stack_all_orgs ON leaky_rls_stack AS RESTRICTIVE FOR SELECT
    USING (org_id IN (SELECT o.org_id FROM leaky_rls_stack_orgs o));
CREATE POLICY stack_org_1 ON leaky_rls_stack FOR SELECT
    USING (leaky_rls_stack_org_check(org_id));

BEGIN;
SET LOCAL ROLE authenticated;
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id FROM leaky_rls_stack s
WHERE body @@@ 'sheriff' AND (SELECT s.secret::int) > 0
ORDER BY id;
SELECT id FROM leaky_rls_stack s
WHERE body @@@ 'sheriff' AND (SELECT s.secret::int) > 0
ORDER BY id;
COMMIT;

DROP TABLE leaky_rls_stack, leaky_rls_stack_orgs CASCADE;
DROP FUNCTION leaky_rls_stack_org_check(int);

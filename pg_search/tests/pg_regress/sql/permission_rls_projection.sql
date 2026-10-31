-- Known search fields must follow reordered projections and native RLS identities.
\i common/common_setup.sql
\set ON_ERROR_STOP on
BEGIN;
SET LOCAL client_min_messages = error;
CREATE ROLE permission_projection_owner NOSUPERUSER NOBYPASSRLS;
CREATE ROLE permission_projection_reader NOSUPERUSER NOBYPASSRLS;
CREATE SCHEMA permission_projection AUTHORIZATION permission_projection_owner;
GRANT USAGE ON SCHEMA permission_projection TO permission_projection_reader;
SET ROLE permission_projection_owner;
CREATE TABLE permission_projection.docs(id integer, dropped text, body text, org integer);
ALTER TABLE permission_projection.docs DROP COLUMN dropped;
INSERT INTO permission_projection.docs VALUES (1, 'alpha', 1), (2, 'alpha', 2), (3, 'beta', 1);
CREATE INDEX docs_idx ON permission_projection.docs USING paradedb(id, body, org);
GRANT SELECT ON permission_projection.docs TO permission_projection_reader;
ALTER TABLE permission_projection.docs ENABLE ROW LEVEL SECURITY;
CREATE POLICY visible ON permission_projection.docs USING (org = 1);
CREATE VIEW permission_projection.full_view AS SELECT org, body, id FROM permission_projection.docs;
CREATE VIEW permission_projection.hidden_view AS SELECT id FROM permission_projection.docs;
GRANT SELECT ON permission_projection.full_view, permission_projection.hidden_view TO permission_projection_reader;
RESET ROLE;
PREPARE projection_derived AS
SELECT array_agg(id ORDER BY id) FROM (SELECT org, id, body FROM permission_projection.docs OFFSET 0) d
WHERE id @@@ pdb.parse('body:alpha');
PREPARE projection_cte AS
WITH d AS MATERIALIZED (SELECT body, org, id FROM permission_projection.docs)
SELECT array_agg(id ORDER BY id) FROM d WHERE id @@@ pdb.parse('body:alpha');
PREPARE projection_view AS
SELECT array_agg(id ORDER BY id) FROM permission_projection.full_view WHERE id @@@ pdb.parse('body:alpha');
SET ROLE permission_projection_reader;
EXECUTE projection_derived;
EXECUTE projection_cte;
EXECUTE projection_view;
DO $$ DECLARE p jsonb; BEGIN
  EXECUTE $q$EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, FORMAT JSON)
    EXECUTE projection_cte$q$ INTO p;
  ASSERT jsonb_path_exists(p, '$.** ? (@."Node Type" == "CTE Scan")'),
    'materialized CTE must execute rather than being flattened';
  ASSERT jsonb_path_exists(p, '$.** ? (@."Node Type" == "Seq Scan" && @."Actual Rows" == 2)'),
    'source scan must retain native RLS filtering';
END $$;
RESET ROLE;
SET ROLE permission_projection_owner;
EXECUTE projection_derived;
EXECUTE projection_cte;
-- Owner bypass is determined by PostgreSQL, rather than the relrowsecurity flag.
SELECT array_agg(id ORDER BY id) = ARRAY[1,2] AS owner_hidden_source
FROM permission_projection.hidden_view WHERE id @@@ pdb.parse('body:alpha');
ALTER TABLE permission_projection.docs FORCE ROW LEVEL SECURITY;
EXECUTE projection_derived;
EXECUTE projection_cte;
DO $$ BEGIN
  BEGIN
    PERFORM id FROM permission_projection.hidden_view WHERE id @@@ pdb.parse('body:alpha');
    RAISE EXCEPTION 'FORCE RLS must prevent the extra-source route';
  EXCEPTION WHEN insufficient_privilege THEN NULL;
  END;
END $$;
RESET ROLE;
DEALLOCATE projection_derived;
DEALLOCATE projection_cte;
DEALLOCATE projection_view;
-- Policy SubLinks follow the same liveness rules as ordinary expressions.
SET ROLE permission_projection_owner;
CREATE TABLE permission_projection.policy_docs(id integer, body text);
INSERT INTO permission_projection.policy_docs VALUES (1, 'alpha'), (2, 'beta');
CREATE INDEX policy_docs_idx ON permission_projection.policy_docs USING paradedb(id, body);
GRANT SELECT ON permission_projection.policy_docs TO permission_projection_reader;
ALTER TABLE permission_projection.policy_docs ENABLE ROW LEVEL SECURITY;
CREATE POLICY visible ON permission_projection.policy_docs USING (
  CASE WHEN false THEN EXISTS (
    SELECT id FROM permission_projection.hidden_view
    WHERE id @@@ pdb.parse('body:alpha')
  ) ELSE true END
);
RESET ROLE;
PREPARE projection_policy_plain AS
SELECT array_agg(id ORDER BY id) FROM permission_projection.policy_docs;
PREPARE projection_policy_search AS
SELECT array_agg(id ORDER BY id) FROM permission_projection.policy_docs
WHERE id @@@ pdb.parse('body:alpha');
SET ROLE permission_projection_reader;
EXECUTE projection_policy_plain;
EXECUTE projection_policy_search;
DO $$ BEGIN
  EXECUTE 'EXPLAIN EXECUTE projection_policy_plain';
  EXECUTE 'EXPLAIN EXECUTE projection_policy_search';
END $$;
RESET ROLE;
-- A live hidden-source policy must still hit the existing RLS guard.
ALTER POLICY visible ON permission_projection.policy_docs USING (EXISTS (
  SELECT id FROM permission_projection.hidden_view
  WHERE id @@@ pdb.parse('body:alpha')
));
SET ROLE permission_projection_reader;
DO $$ DECLARE q text; BEGIN
  FOREACH q IN ARRAY ARRAY['EXECUTE projection_policy_plain', 'EXECUTE projection_policy_search'] LOOP
    BEGIN
      EXECUTE q;
      RAISE EXCEPTION 'live hidden-source policy must be rejected';
    EXCEPTION WHEN insufficient_privilege THEN
      ASSERT SQLERRM = 'opaque search field is not exposed by its row-security source', SQLERRM;
    END;
  END LOOP;
END $$;
RESET ROLE;
-- Keeping live policy checks must also preserve granted view identities.
ALTER POLICY visible ON permission_projection.policy_docs USING (EXISTS (
  SELECT id FROM permission_projection.full_view
  WHERE id @@@ pdb.parse('body:alpha')
));
SET ROLE permission_projection_reader;
EXECUTE projection_policy_plain;
EXECUTE projection_policy_search;
RESET ROLE;
DEALLOCATE projection_policy_plain;
DEALLOCATE projection_policy_search;
ROLLBACK;

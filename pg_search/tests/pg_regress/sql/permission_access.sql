\set VERBOSITY terse
-- Direct APIs share a read policy; maintenance has a separate privilege policy.
-- Every negative assertion checks 42501, so a malformed call cannot pass.
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql
BEGIN;
CREATE ROLE permission_access_reader NOSUPERUSER NOBYPASSRLS;
CREATE SCHEMA permission_access;
GRANT USAGE ON SCHEMA permission_access TO permission_access_reader;
CREATE TABLE permission_access.docs(id integer PRIMARY KEY, email text);
INSERT INTO permission_access.docs VALUES (1, 'first'), (2, 'second');
CREATE INDEX docs_idx ON permission_access.docs USING paradedb (id, (email::pdb.literal));
SELECT 'permission_access.docs_idx'::regclass::oid AS indexoid \gset

CREATE FUNCTION permission_access.assert_denied(queries text[]) RETURNS integer
LANGUAGE plpgsql SECURITY INVOKER AS $$
DECLARE q text; n integer := 0;
BEGIN
  FOREACH q IN ARRAY queries LOOP
    BEGIN
      EXECUTE q;
      RAISE EXCEPTION 'unexpected successful access: %', q;
    EXCEPTION WHEN insufficient_privilege THEN n := n + 1;
    END;
  END LOOP;
  RETURN n;
END $$;
CREATE FUNCTION permission_access.read_queries() RETURNS text[]
LANGUAGE sql IMMUTABLE AS $$ SELECT ARRAY[
  'SELECT * FROM paradedb.schema(''permission_access.docs_idx'')',
  'SELECT paradedb.index_fields(''permission_access.docs_idx'')',
  'SELECT paradedb.layer_sizes(''permission_access.docs_idx'')',
  'SELECT paradedb.background_layer_sizes(''permission_access.docs_idx'')',
  'SELECT paradedb.combined_layer_sizes(''permission_access.docs_idx'')',
  'SELECT paradedb.index_created_at(''permission_access.docs_idx'')',
  'SELECT paradedb.index_created_by(''permission_access.docs_idx'')',
  'SELECT * FROM paradedb.merge_info(''permission_access.docs_idx'')',
  'SELECT paradedb.is_merging(''permission_access.docs_idx'')',
  'SELECT * FROM paradedb.vacuum_info(''permission_access.docs_idx'')',
  'SELECT * FROM paradedb.fsm_info(''permission_access.docs_idx'')',
  'SELECT paradedb.fsm_size(''permission_access.docs_idx'')',
  'SELECT * FROM paradedb.index_info(''permission_access.docs_idx'')',
  'SELECT * FROM pdb.index_segments(''permission_access.docs_idx'')',
  'SELECT * FROM paradedb.vector_info(''permission_access.docs_idx'', ''missing'')',
  'SELECT * FROM paradedb.vector_config(''permission_access.docs_idx'', ''missing'')',
  'SELECT * FROM paradedb.vector_clusters(''permission_access.docs_idx'', ''missing'')',
  'SELECT paradedb.find_ctid(''permission_access.docs_idx'', ''(0,1)''::tid)',
  'SELECT * FROM paradedb.validate_checksum(''permission_access.docs_idx'')',
  'SELECT * FROM paradedb.storage_info(''permission_access.docs_idx'')',
  'SELECT * FROM paradedb.page_info(''permission_access.docs_idx'', 0)',
  'SELECT * FROM pdb.verify_index(''permission_access.docs_idx'')',
  'SELECT paradedb.aggregate(''permission_access.docs_idx'', paradedb.all(), ''{"a":{"terms":{"field":"email"}}}'')'
] $$;

-- Audit every regclass overload, including functions that do not read an index.
-- A new signature must be classified and its access/lock policy covered explicitly.
-- Exceptions: with_index constructs query input; deprecated bgmerger_state,
-- reset_bgworker_state and both force_merge overloads read no index data.
-- SQL signatures cannot distinguish IndexOid from pgrx's implicitly locking PgRelation;
-- the lock-order tests independently check that argument decoding takes no index lock.
SELECT n.nspname || '.' || p.proname || '(' ||
       pg_get_function_identity_arguments(p.oid) || ')' AS regclass_function
FROM pg_proc p
JOIN pg_namespace n ON n.oid = p.pronamespace
JOIN pg_depend d ON d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e'
JOIN pg_extension e ON e.oid = d.refobjid AND e.extname = 'pg_search'
WHERE 'regclass'::regtype::oid = ANY (p.proargtypes::oid[])
ORDER BY 1;

SET ROLE permission_access_reader;
SELECT permission_access.assert_denied(permission_access.read_queries()) AS no_grant_denials;
SELECT permission_access.assert_denied(ARRAY[
  'SELECT * FROM paradedb.vector_estimator_info(''permission_access.docs_idx'', ''missing'')'
]) AS vector_estimator_denial;
SELECT count(*) AS hidden_inventory FROM pdb.indexes() WHERE schemaname = 'permission_access';
SELECT count(*) AS hidden_bulk_checks FROM pdb.verify_all_indexes(schema_pattern => 'permission_access');
RESET ROLE;

GRANT SELECT(id) ON permission_access.docs TO permission_access_reader;
SET ROLE permission_access_reader;
SELECT count(id) AS ordinary_id_access FROM permission_access.docs;
SELECT permission_access.assert_denied(permission_access.read_queries()) AS column_grant_denials;
RESET ROLE;

GRANT SELECT ON permission_access.docs TO permission_access_reader;
SET ROLE permission_access_reader;
SELECT count(*) AS allowed_fields FROM paradedb.schema('permission_access.docs_idx');
SELECT paradedb.fsm_size('permission_access.docs_idx') > 0 AS allowed_fsm_size;
SELECT count(*) >= 0 AS allowed_fsm_info FROM paradedb.fsm_info('permission_access.docs_idx');
SELECT sum(num_docs) AS allowed_docs FROM pdb.index_segments('permission_access.docs_idx');
SELECT count(*) AS visible_inventory FROM pdb.indexes() WHERE schemaname = 'permission_access';
SELECT bool_and(passed) AS allowed_verification FROM pdb.verify_index('permission_access.docs_idx');
SELECT permission_access.assert_denied(ARRAY[
  'SELECT * FROM paradedb.merge_lock_garbage_collect(''permission_access.docs_idx'')'
]) AS reader_cannot_maintain;
RESET ROLE;
SELECT count(*) AS owner_maintenance FROM paradedb.merge_lock_garbage_collect('permission_access.docs_idx');

-- Maintenance follows REINDEX: MAINTAIN from PostgreSQL 17, ownership before.
-- Version-dependent grants share one expected output across supported releases.
CREATE ROLE permission_access_maintainer NOSUPERUSER NOBYPASSRLS INHERIT;
CREATE ROLE permission_access_table_maintainer NOSUPERUSER NOBYPASSRLS;
GRANT USAGE ON SCHEMA permission_access TO permission_access_maintainer, permission_access_table_maintainer;
SELECT current_setting('server_version_num')::int >= 170000 AS maintain_privilege_exists \gset
\if :maintain_privilege_exists
GRANT pg_maintain TO permission_access_maintainer;
GRANT MAINTAIN ON permission_access.docs TO permission_access_table_maintainer;
\endif
CREATE FUNCTION permission_access.try_maintain() RETURNS boolean
LANGUAGE plpgsql SECURITY INVOKER AS $$
BEGIN
  PERFORM paradedb.merge_lock_garbage_collect('permission_access.docs_idx');
  RETURN true;
EXCEPTION WHEN insufficient_privilege THEN RETURN false;
END $$;
SET ROLE permission_access_maintainer;
SELECT permission_access.try_maintain() = :'maintain_privilege_exists'::boolean AS pg_maintain_member_follows_reindex;
SELECT permission_access.assert_denied(permission_access.read_queries()) AS maintainer_read_denials;
RESET ROLE;
SET ROLE permission_access_table_maintainer;
SELECT permission_access.try_maintain() = :'maintain_privilege_exists'::boolean AS table_maintain_grant_follows_reindex;
RESET ROLE;
\if :maintain_privilege_exists
REVOKE pg_maintain FROM permission_access_maintainer;
REVOKE MAINTAIN ON permission_access.docs FROM permission_access_table_maintainer;
\endif
SET ROLE permission_access_maintainer;
SELECT NOT permission_access.try_maintain() AS revoked_role_denies_maintenance;
RESET ROLE;
SET ROLE permission_access_table_maintainer;
SELECT NOT permission_access.try_maintain() AS revoked_table_grant_denies_maintenance;
RESET ROLE;

REVOKE USAGE ON SCHEMA permission_access FROM permission_access_reader;
SET ROLE permission_access_reader;
-- Numeric regclass avoids schema name resolution: denial must come from access policy.
SAVEPOINT no_schema;
SELECT count(*) FROM pdb.index_segments(:indexoid::regclass);
ROLLBACK TO SAVEPOINT no_schema;
RESET ROLE;
GRANT USAGE ON SCHEMA permission_access TO permission_access_reader;
ALTER TABLE permission_access.docs ENABLE ROW LEVEL SECURITY;
SET ROLE permission_access_reader;
SELECT count(*) AS ordinary_rls_rows FROM permission_access.docs;
-- Name the backing table when a multi-index diagnostic hits active RLS.
SAVEPOINT named_rls_error;
SELECT * FROM pdb.index_segments('permission_access.docs_idx');
ROLLBACK TO SAVEPOINT named_rls_error;
SELECT permission_access.assert_denied(permission_access.read_queries()) AS rls_denials;
SELECT count(*) AS rls_inventory FROM pdb.indexes() WHERE schemaname = 'permission_access';
SET LOCAL row_security = off;
SELECT permission_access.assert_denied(permission_access.read_queries()) AS rls_off_denials;
RESET row_security;
RESET ROLE;

CREATE FUNCTION permission_access.definer_segments() RETURNS bigint
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, pg_temp AS $$
  SELECT count(*) FROM pdb.index_segments('permission_access.docs_idx'::regclass)
$$;
SET ROLE permission_access_reader;
SELECT permission_access.definer_segments() AS effective_definer_identity;
RESET ROLE;
ALTER ROLE permission_access_reader BYPASSRLS;
SET ROLE permission_access_reader;
SELECT sum(num_docs) AS bypassrls_docs FROM pdb.index_segments('permission_access.docs_idx');
RESET ROLE;
ALTER ROLE permission_access_reader NOBYPASSRLS;
ALTER TABLE permission_access.docs OWNER TO permission_access_reader;
SET ROLE permission_access_reader;
SELECT sum(num_docs) AS owner_rls_docs FROM pdb.index_segments('permission_access.docs_idx');
ALTER TABLE permission_access.docs FORCE ROW LEVEL SECURITY;
SELECT permission_access.assert_denied(permission_access.read_queries()) AS forced_owner_denials;
SELECT count(*) AS forced_owner_maintenance FROM paradedb.merge_lock_garbage_collect('permission_access.docs_idx');
RESET ROLE;

CREATE TABLE permission_access.parent(id integer) PARTITION BY RANGE(id);
CREATE TABLE permission_access.child PARTITION OF permission_access.parent FOR VALUES FROM (0) TO (10);
INSERT INTO permission_access.parent VALUES (1);
CREATE INDEX parent_idx ON permission_access.parent USING paradedb (id);
SELECT indexrelid AS childindexoid FROM pg_index
WHERE indrelid = 'permission_access.child'::regclass \gset
GRANT SELECT ON permission_access.parent TO permission_access_reader;
SET ROLE permission_access_reader;
SELECT sum(num_docs) AS granted_parent_docs FROM pdb.index_segments('permission_access.parent_idx');
SAVEPOINT direct_child;
SELECT count(*) FROM pdb.index_segments(:childindexoid::regclass);
ROLLBACK TO SAVEPOINT direct_child;
RESET ROLE;
-- Exact standalone wrapper pair: a non-superuser table owner and no-grant caller.
CREATE ROLE permission_access_owner NOSUPERUSER NOBYPASSRLS NOINHERIT;
CREATE ROLE permission_access_ungranted NOSUPERUSER NOBYPASSRLS NOINHERIT;
CREATE SCHEMA permission_identity AUTHORIZATION permission_access_owner;
GRANT USAGE ON SCHEMA permission_identity, permission_access TO permission_access_ungranted;
CREATE TABLE permission_identity.docs(id integer PRIMARY KEY, email text);
INSERT INTO permission_identity.docs VALUES (1, 'first'), (2, 'second');
CREATE INDEX docs_idx ON permission_identity.docs USING paradedb (id, (email::pdb.literal));
ALTER TABLE permission_identity.docs OWNER TO permission_access_owner;
ALTER TABLE permission_identity.docs ENABLE ROW LEVEL SECURITY;
CREATE FUNCTION permission_identity.definer_segments() RETURNS bigint
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, pg_temp AS $$
  SELECT coalesce(sum(num_docs), 0)::bigint
  FROM pdb.index_segments('permission_identity.docs_idx'::regclass)
$$;
CREATE FUNCTION permission_identity.invoker_segments() RETURNS bigint
LANGUAGE sql SECURITY INVOKER SET search_path = pg_catalog, pg_temp AS $$
  SELECT coalesce(sum(num_docs), 0)::bigint
  FROM pdb.index_segments('permission_identity.docs_idx'::regclass)
$$;
ALTER FUNCTION permission_identity.definer_segments() OWNER TO permission_access_owner;
ALTER FUNCTION permission_identity.invoker_segments() OWNER TO permission_access_owner;
REVOKE EXECUTE ON FUNCTION permission_identity.definer_segments(), permission_identity.invoker_segments() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION permission_identity.definer_segments(), permission_identity.invoker_segments() TO permission_access_ungranted;
SELECT bool_and(p.proowner = c.relowner) AS wrappers_owned_by_table_owner
FROM pg_proc p JOIN pg_class c ON c.oid = 'permission_identity.docs'::regclass
WHERE p.pronamespace = 'permission_identity'::regnamespace;
SET ROLE permission_access_ungranted;
SELECT has_table_privilege(current_user, 'permission_identity.docs', 'SELECT') AS table_select,
 pg_has_role(current_user, 'pg_stat_scan_tables', 'USAGE') AS diagnostic_privileges;
SELECT permission_identity.definer_segments() AS no_grant_definer_docs;
SELECT permission_access.assert_denied(ARRAY[
 'SELECT permission_identity.invoker_segments()',
 'SELECT * FROM pdb.index_segments(''permission_identity.docs_idx'')'
]) AS no_grant_invoker_and_direct_denials;
RESET ROLE;
-- The definer is a real table owner, not a superuser or BYPASSRLS role.
ALTER TABLE permission_identity.docs FORCE ROW LEVEL SECURITY;
SET ROLE permission_access_ungranted;
SELECT permission_access.assert_denied(ARRAY[
 'SELECT permission_identity.definer_segments()'
]) AS forced_rls_definer_denial;
RESET ROLE;
ROLLBACK;

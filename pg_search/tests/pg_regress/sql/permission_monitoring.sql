-- Trusted diagnostic metadata follows pg_stat_scan_tables, inherited by pg_monitor.
-- Content and maintenance APIs retain their existing permission policies.
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql
BEGIN;
CREATE ROLE permission_monitoring_monitor;
CREATE ROLE permission_monitoring_scan;
CREATE ROLE permission_monitoring_inherited;
CREATE ROLE permission_monitoring_noinherit NOINHERIT;
CREATE ROLE permission_monitoring_reader;
GRANT pg_monitor TO permission_monitoring_monitor;
GRANT pg_stat_scan_tables TO permission_monitoring_scan, permission_monitoring_noinherit;
GRANT permission_monitoring_monitor TO permission_monitoring_inherited;
CREATE SCHEMA permission_monitoring;
CREATE SCHEMA permission_monitoring_private;
GRANT USAGE ON SCHEMA permission_monitoring TO
  permission_monitoring_monitor, permission_monitoring_scan,
  permission_monitoring_inherited, permission_monitoring_noinherit,
  permission_monitoring_reader;
CREATE TABLE permission_monitoring.docs(id integer PRIMARY KEY, body text);
CREATE TABLE permission_monitoring.rls_docs(id integer PRIMARY KEY, body text);
CREATE TABLE permission_monitoring_private.docs(id integer PRIMARY KEY, body text);
INSERT INTO permission_monitoring.docs VALUES (1, 'first'), (2, 'second');
INSERT INTO permission_monitoring.rls_docs VALUES (1, 'visible'), (2, 'hidden');
INSERT INTO permission_monitoring_private.docs VALUES (1, 'private');
CREATE INDEX docs_idx ON permission_monitoring.docs USING paradedb (id, body);
CREATE INDEX rls_docs_idx ON permission_monitoring.rls_docs USING paradedb (id, body);
CREATE INDEX docs_idx ON permission_monitoring_private.docs USING paradedb (id, body);
SELECT 'permission_monitoring_private.docs_idx'::regclass::oid AS private_index_oid \gset
SELECT 'permission_monitoring.rls_docs'::regclass::oid AS rls_table_oid \gset
ALTER TABLE permission_monitoring.rls_docs ENABLE ROW LEVEL SECURITY;
ALTER TABLE permission_monitoring.rls_docs FORCE ROW LEVEL SECURITY;
CREATE POLICY only_one ON permission_monitoring.rls_docs USING (id = 1);

CREATE FUNCTION permission_monitoring.assert_denied(queries text[]) RETURNS integer
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
CREATE FUNCTION permission_monitoring.metadata_queries() RETURNS text[]
LANGUAGE sql IMMUTABLE AS $$ SELECT ARRAY[
 'SELECT * FROM paradedb.schema(''permission_monitoring.docs_idx'')',
 'SELECT * FROM paradedb.index_info(''permission_monitoring.docs_idx'')',
 'SELECT * FROM paradedb.index_info(''permission_monitoring.rls_docs_idx'', true)',
 'SELECT * FROM pdb.index_segments(''permission_monitoring.docs_idx'')',
 'SELECT * FROM pdb.index_segments(''permission_monitoring.rls_docs_idx'')'
] $$;
CREATE FUNCTION permission_monitoring.content_queries() RETURNS text[]
LANGUAGE sql IMMUTABLE AS $$ SELECT ARRAY[
 'SELECT * FROM permission_monitoring.docs',
 'SELECT * FROM permission_monitoring.rls_docs',
 'SELECT paradedb.find_ctid(''permission_monitoring.docs_idx'', ''(0,1)''::tid)',
 'SELECT paradedb.aggregate(''permission_monitoring.docs_idx'', paradedb.all(), ''{"a":{"terms":{"field":"body"}}}'')',
 'SELECT * FROM pdb.verify_index(''permission_monitoring.docs_idx'')',
 'SELECT * FROM paradedb.merge_lock_garbage_collect(''permission_monitoring.docs_idx'')',
 'SELECT * FROM paradedb.vector_estimator_info(''permission_monitoring.docs_idx'', ''missing'')'
] $$;

SET ROLE permission_monitoring_reader;
SELECT permission_monitoring.assert_denied(permission_monitoring.metadata_queries()) AS nonmember_denials;
SELECT count(*) AS hidden_inventory FROM pdb.indexes() WHERE schemaname = 'permission_monitoring';
RESET ROLE;
GRANT SELECT(id) ON permission_monitoring.docs TO permission_monitoring_reader;
SET ROLE permission_monitoring_reader;
SELECT permission_monitoring.assert_denied(permission_monitoring.metadata_queries()) AS column_only_denials;
RESET ROLE;

-- Direct pg_stat_scan_tables membership, with no table SELECT.
SET ROLE permission_monitoring_scan;
SELECT count(*) AS schema_fields FROM paradedb.schema('permission_monitoring.docs_idx');
SELECT sum(num_docs) AS ordinary_docs FROM pdb.index_segments('permission_monitoring.docs_idx');
SELECT sum(num_docs) AS global_rls_docs FROM pdb.index_segments('permission_monitoring.rls_docs_idx');
SELECT sum(num_docs) AS index_info_docs FROM paradedb.index_info('permission_monitoring.docs_idx');
SELECT sum(num_docs) AS invisible_rls_metadata_docs
FROM paradedb.index_info('permission_monitoring.rls_docs_idx', true);
SELECT count(*) AS visible_indexes, sum(total_docs) AS inventory_docs
FROM pdb.indexes() WHERE schemaname = 'permission_monitoring';
SELECT permission_monitoring.assert_denied(permission_monitoring.content_queries()) AS content_denials;
SELECT count(*) AS hidden_bulk_verification
FROM pdb.verify_all_indexes(schema_pattern => 'permission_monitoring');
-- OID-based trusted diagnostics work even without USAGE on the table's schema.
SELECT has_schema_privilege(current_user, 'permission_monitoring_private', 'USAGE') AS private_schema_usage;
SELECT sum(num_docs) AS private_docs FROM pdb.index_segments(:private_index_oid::regclass);
SELECT count(*) AS private_inventory FROM pdb.indexes() WHERE schemaname = 'permission_monitoring_private';
RESET ROLE;

-- pg_monitor inherits the diagnostic privilege, including through another role.
SET ROLE permission_monitoring_inherited;
SELECT pg_has_role(current_user, 'pg_stat_scan_tables', 'USAGE') AS inherited_diagnostic;
SELECT sum(num_docs) AS inherited_rls_docs FROM pdb.index_segments('permission_monitoring.rls_docs_idx');
SELECT permission_monitoring.assert_denied(permission_monitoring.content_queries()) AS inherited_content_denials;
PREPARE permission_monitoring_cached AS
 SELECT sum(num_docs) FROM pdb.index_segments('permission_monitoring.rls_docs_idx');
EXECUTE permission_monitoring_cached;
RESET ROLE;
REVOKE pg_monitor FROM permission_monitoring_monitor;
SET ROLE permission_monitoring_inherited;
SELECT permission_monitoring.assert_denied(ARRAY['EXECUTE permission_monitoring_cached']) AS revoked_prepared_denial;
SELECT count(*) AS revoked_inventory FROM pdb.indexes() WHERE schemaname = 'permission_monitoring';
RESET ROLE;
GRANT pg_monitor TO permission_monitoring_monitor;
SET ROLE permission_monitoring_inherited;
EXECUTE permission_monitoring_cached;
DEALLOCATE permission_monitoring_cached;
RESET ROLE;

-- Membership without inherited privileges must not authorize metadata.
SET ROLE permission_monitoring_noinherit;
SELECT pg_has_role(current_user, 'pg_stat_scan_tables', 'MEMBER') AS is_member,
 pg_has_role(current_user, 'pg_stat_scan_tables', 'USAGE') AS has_privileges;
SELECT permission_monitoring.assert_denied(permission_monitoring.metadata_queries()) AS noinherit_denials;
RESET ROLE;

-- The normal SELECT/no-active-RLS route still works for nonmembers.
GRANT SELECT ON permission_monitoring.docs, permission_monitoring.rls_docs TO permission_monitoring_reader;
SET ROLE permission_monitoring_reader;
SELECT sum(num_docs) AS reader_docs FROM pdb.index_segments('permission_monitoring.docs_idx');
SELECT count(*) AS policy_visible_rows FROM permission_monitoring.rls_docs;
SELECT permission_monitoring.assert_denied(ARRAY[
 'SELECT * FROM pdb.index_segments(''permission_monitoring.rls_docs_idx'')'
]) AS reader_rls_denial;
SELECT count(*) AS reader_inventory FROM pdb.indexes() WHERE schemaname = 'permission_monitoring';
RESET ROLE;

-- Function EXECUTE remains a separate ACL, just as it is in PostgreSQL.
REVOKE EXECUTE ON FUNCTION pdb.index_segments(regclass) FROM PUBLIC;
SET ROLE permission_monitoring_scan;
SELECT permission_monitoring.assert_denied(ARRAY[
 'SELECT * FROM pdb.index_segments(''permission_monitoring.docs_idx'')'
]) AS execute_acl_denial;
RESET ROLE;
GRANT EXECUTE ON FUNCTION pdb.index_segments(regclass) TO permission_monitoring_scan;
SET ROLE permission_monitoring_scan;
SELECT sum(num_docs) AS explicit_execute_docs FROM pdb.index_segments('permission_monitoring.rls_docs_idx');
RESET ROLE;
ROLLBACK;

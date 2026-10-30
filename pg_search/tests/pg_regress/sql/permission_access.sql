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

ALTER TABLE permission_access.docs ENABLE ROW LEVEL SECURITY;
SET ROLE permission_access_reader;
SELECT count(*) AS ordinary_rls_rows FROM permission_access.docs;
-- Name the backing table when a multi-index diagnostic hits active RLS.
SAVEPOINT named_rls_error;
SELECT * FROM pdb.index_segments('permission_access.docs_idx');
ROLLBACK TO SAVEPOINT named_rls_error;
SELECT permission_access.assert_denied(permission_access.read_queries()) AS rls_denials;
SELECT count(*) AS rls_inventory FROM pdb.indexes() WHERE schemaname = 'permission_access';
RESET ROLE;
ROLLBACK;

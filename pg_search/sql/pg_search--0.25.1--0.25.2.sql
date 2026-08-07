\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.2'" to load this file. \quit

-- 0.25.1 -> 0.25.2: harden the tokenizer typmod cache permissions.
--
-- paradedb._typmod_cache and its sequence were previously created with
-- `GRANT ALL ... TO PUBLIC`. That is broader than the extension needs: ordinary
-- roles only read the table (via SPI in load_typmod) and insert through the
-- SECURITY DEFINER function paradedb._save_typmod. The extra UPDATE/DELETE/
-- TRUNCATE and sequence-write privileges let any role silently repoint or orphan
-- the typmod IDs that other users' ParadeDB indexes resolve their tokenizer
-- configuration through. Narrow PUBLIC down to SELECT and keep all writes with
-- the table owner.
--
-- SELECT stays on the sequence as well: both relations are registered with
-- pg_extension_config_dump, so pg_dump reads the sequence's last_value directly
-- and a non-owner dump role would otherwise fail. SELECT on a sequence does not
-- permit nextval/setval, which is where the hardening actually matters; the
-- sequence is only ever advanced from inside _save_typmod, as its owner.
REVOKE ALL ON TABLE paradedb._typmod_cache FROM PUBLIC;
REVOKE ALL ON SEQUENCE paradedb._typmod_cache_id_seq FROM PUBLIC;
GRANT SELECT ON TABLE paradedb._typmod_cache TO PUBLIC;
GRANT SELECT ON SEQUENCE paradedb._typmod_cache_id_seq TO PUBLIC;

-- Pin the SECURITY DEFINER function's search_path so its name resolution can't be
-- redirected by a caller-controlled search_path. The body only touches the
-- schema-qualified paradedb._typmod_cache plus pg_catalog operators, so pinning
-- it leaves behaviour unchanged.
--
-- The body below is identical to the one in
-- pg_search/src/api/tokenizers/typmod/mod.rs. Only the header is spelled
-- differently (`pg_catalog.int4` rather than `integer`, and the options after
-- the body rather than before it), matching what SchemaBot generates for this
-- change so that check_migration_diff.py sees the same statements.
DROP FUNCTION IF EXISTS paradedb._save_typmod(typmod_in text[]);
CREATE OR REPLACE FUNCTION paradedb._save_typmod(typmod_in text[]) RETURNS pg_catalog.int4 AS $$
DECLARE
    v_id integer;
BEGIN
    INSERT INTO paradedb._typmod_cache (typmod)
    VALUES (typmod_in)
    ON CONFLICT (typmod) DO NOTHING
    RETURNING id INTO v_id;

    IF v_id IS NOT NULL THEN
        RETURN v_id;
    END IF;

    -- someone else inserted it concurrently, go read it again
    SELECT id INTO v_id
    FROM paradedb._typmod_cache
    WHERE typmod = typmod_in;

    IF v_id IS NULL THEN
        RAISE EXCEPTION 'typmod "%" not found after upsert', typmod_in;
    END IF;

    RETURN v_id;
END;
$$ LANGUAGE plpgsql PARALLEL UNSAFE SECURITY DEFINER SET search_path TO 'pg_catalog', 'pg_temp' STRICT VOLATILE;

-- The paradedb and pdb schemas were created with GRANT ALL TO PUBLIC, which
-- includes CREATE. PUBLIC only needs USAGE to reference the extension's objects,
-- and every object in these schemas is created by the extension owner, so revoke
-- CREATE: no ordinary role should be able to squat objects inside the
-- extension's own schemas.
REVOKE CREATE ON SCHEMA paradedb FROM PUBLIC;
REVOKE CREATE ON SCHEMA pdb FROM PUBLIC;

-- pg_search/src/api/operator/eqeqeq.rs:38
-- pg_search::api::operator::eqeqeq::term_search_query_input
CREATE  FUNCTION "term_search_query_input"(
	"field" FieldName, /* FieldName */
	"query" pdb.Query /* pdb :: Query */
) RETURNS SearchQueryInput /* SearchQueryInput */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_search_query_input_wrapper';

-- Rename paradedb.create_bm25_test_table to paradedb.create_paradedb_test_table.
-- The bm25 name predates the access method rename to `paradedb`, and this
-- procedure was one of the last user-facing identifiers still carrying it. The
-- signature and behaviour are unchanged, including the 'bm25_test_table'
-- default table name, so an existing call that passes no table_name still
-- creates exactly the same table.
DROP PROCEDURE IF EXISTS paradedb.create_bm25_test_table(table_name pg_catalog."varchar", schema_name pg_catalog."varchar", table_type paradedb.testtable);
CREATE OR REPLACE PROCEDURE paradedb.create_paradedb_test_table(table_name VARCHAR DEFAULT 'bm25_test_table', schema_name VARCHAR DEFAULT 'paradedb', table_type paradedb.TestTable DEFAULT 'Items')
LANGUAGE c AS 'MODULE_PATHNAME', 'create_paradedb_test_table_wrapper';

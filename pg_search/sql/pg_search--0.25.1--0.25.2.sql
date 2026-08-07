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
REVOKE ALL ON TABLE paradedb._typmod_cache FROM PUBLIC;
REVOKE ALL ON SEQUENCE paradedb._typmod_cache_id_seq FROM PUBLIC;
GRANT SELECT ON TABLE paradedb._typmod_cache TO PUBLIC;

-- Pin the SECURITY DEFINER function's search_path so its name resolution can't be
-- redirected by a caller-controlled search_path. The body only touches the
-- schema-qualified paradedb._typmod_cache plus pg_catalog operators, so pinning
-- it leaves behaviour unchanged.
ALTER FUNCTION paradedb._save_typmod(text[]) SET search_path = pg_catalog, pg_temp;

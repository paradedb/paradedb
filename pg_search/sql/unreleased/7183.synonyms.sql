-- Query-time synonyms: the set table, the trigger that stamps every update, and the
-- function that shows an expansion.

CREATE FUNCTION "synonym_sets_bump_version"()
	RETURNS TRIGGER
	LANGUAGE c
	AS 'MODULE_PATHNAME', 'synonym_sets_bump_version_wrapper';

CREATE TABLE paradedb.synonym_sets (
    name       text NOT NULL PRIMARY KEY CHECK (name <> 'none'),
    rules      text[] NOT NULL,
    version    bigint NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.now()
);
SELECT pg_catalog.pg_extension_config_dump('paradedb.synonym_sets', '');

-- Every role that runs a query reads the set its index names, so PUBLIC gets SELECT. Who may
-- write a set is the DBA's decision, through ordinary table grants.
GRANT SELECT ON TABLE paradedb.synonym_sets TO PUBLIC;

CREATE TRIGGER synonym_sets_bump_version
    BEFORE UPDATE ON paradedb.synonym_sets
    FOR EACH ROW EXECUTE FUNCTION paradedb.synonym_sets_bump_version();

CREATE  FUNCTION "synonym_expansion"(
	"set_name" TEXT, /* & str */
	"tokenizer_setting" jsonb, /* JsonB */
	"input_text" TEXT /* & str */
) RETURNS TABLE (
	"position" INT,  /* i32 */
	"token" TEXT,  /* String */
	"alternatives" TEXT[]  /* Vec < String > */
)
STRICT STABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'synonym_expansion_wrapper';

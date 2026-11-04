-- Compiled synonym sets. One row per set and analyzer, filled by the builder on the first query
-- that needs the pair.
-- depends-on: 7220

CREATE FUNCTION pdb."synonym_fst_build"(
	"set_name" TEXT, /* & str */
	"analyzer" jsonb /* JsonB */
) RETURNS bool /* bool */
STRICT VOLATILE SECURITY DEFINER PARALLEL UNSAFE
SET search_path TO pg_catalog, pg_temp
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'synonym_fst_build_wrapper';

-- Derived rows, rebuilt on demand, so they are not dumped. The key is a digest of the analyzer
-- config, since a config with a long stopword list is too big for a btree entry.
CREATE TABLE pdb.synonym_fsts (
    set_name      text NOT NULL REFERENCES pdb.synonym_sets (name) ON DELETE CASCADE,
    analyzer_hash text NOT NULL,
    analyzer      jsonb NOT NULL,
    version       bigint NOT NULL,
    built_with    text NOT NULL,
    fst           bytea NOT NULL,
    alternatives  bytea NOT NULL,
    PRIMARY KEY (set_name, analyzer_hash)
);
GRANT SELECT ON pdb.synonym_fsts TO PUBLIC;

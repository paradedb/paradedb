-- Query-time synonyms: the rule type, the set, rule and compiled-set tables, the triggers that
-- keep them in step, the builder of a compiled set, and the function that shows an expansion.

CREATE FUNCTION pdb."synonym_rule_is_valid"(
	"rule" TEXT /* & str */
) RETURNS bool /* bool */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'synonym_rule_is_valid_wrapper';

CREATE FUNCTION pdb."synonym_rules_bump_version"()
	RETURNS TRIGGER
	LANGUAGE c
	AS 'MODULE_PATHNAME', 'synonym_rules_bump_version_wrapper';

CREATE FUNCTION pdb."synonym_fst_build"(
	"set_name" TEXT, /* & str */
	"analyzer" jsonb /* JsonB */
) RETURNS bool /* bool */
STRICT VOLATILE SECURITY DEFINER PARALLEL UNSAFE
SET search_path TO pg_catalog, pg_temp
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'synonym_fst_build_wrapper';

CREATE FUNCTION pdb."synonym_expansion"(
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

CREATE DOMAIN pdb.synonym_rule AS text
    CHECK (pdb.synonym_rule_is_valid(VALUE));

CREATE TABLE pdb.synonym_sets (
    name       text NOT NULL PRIMARY KEY CHECK (name <> 'none'),
    version    bigint NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.now()
);
CREATE TABLE pdb.synonym_rules (
    set_name   text NOT NULL REFERENCES pdb.synonym_sets (name) ON DELETE CASCADE,
    rule       pdb.synonym_rule NOT NULL
);
CREATE INDEX synonym_rules_set_name_idx ON pdb.synonym_rules (set_name);
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
SELECT pg_catalog.pg_extension_config_dump('pdb.synonym_sets', '');
SELECT pg_catalog.pg_extension_config_dump('pdb.synonym_rules', '');

-- Every role that runs a query reads the set its index names, so PUBLIC gets SELECT. Writing
-- rules takes a grant on pdb.synonym_rules, removing a set a grant on pdb.synonym_sets, and
-- nothing else: both trigger functions and the builder run as the extension owner.
GRANT SELECT ON pdb.synonym_sets, pdb.synonym_rules, pdb.synonym_fsts TO PUBLIC;

-- A set appears with its first rule, so a writer never touches pdb.synonym_sets. The IF NOT
-- EXISTS check keeps a COPY of many rules from paying a speculative insert per row.
CREATE FUNCTION pdb.ensure_synonym_set() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pdb.synonym_sets WHERE name = NEW.set_name) THEN
        INSERT INTO pdb.synonym_sets (name) VALUES (NEW.set_name) ON CONFLICT (name) DO NOTHING;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER synonym_rules_ensure_set
    BEFORE INSERT OR UPDATE OF set_name ON pdb.synonym_rules
    FOR EACH ROW EXECUTE FUNCTION pdb.ensure_synonym_set();

ALTER FUNCTION pdb.synonym_rules_bump_version() SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp;
-- One trigger per event, because a transition table binds to a single event.
CREATE TRIGGER synonym_rules_bump_version_insert
    AFTER INSERT ON pdb.synonym_rules
    REFERENCING NEW TABLE AS new_rules
    FOR EACH STATEMENT EXECUTE FUNCTION pdb.synonym_rules_bump_version();
CREATE TRIGGER synonym_rules_bump_version_update
    AFTER UPDATE ON pdb.synonym_rules
    REFERENCING OLD TABLE AS old_rules NEW TABLE AS new_rules
    FOR EACH STATEMENT EXECUTE FUNCTION pdb.synonym_rules_bump_version();
CREATE TRIGGER synonym_rules_bump_version_delete
    AFTER DELETE ON pdb.synonym_rules
    REFERENCING OLD TABLE AS old_rules
    FOR EACH STATEMENT EXECUTE FUNCTION pdb.synonym_rules_bump_version();
CREATE TRIGGER synonym_rules_bump_version_truncate
    AFTER TRUNCATE ON pdb.synonym_rules
    FOR EACH STATEMENT EXECUTE FUNCTION pdb.synonym_rules_bump_version();

-- Postgres checks EXECUTE on a trigger function at CREATE TRIGGER only, so the triggers above
-- keep firing, and no other role can hang these owner-level functions on a table of its own.
REVOKE EXECUTE ON FUNCTION pdb.ensure_synonym_set(), pdb.synonym_rules_bump_version()
    FROM PUBLIC;

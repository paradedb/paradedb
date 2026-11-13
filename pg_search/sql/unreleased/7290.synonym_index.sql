CREATE FUNCTION pdb."synonym_rule_is_valid"(
    "rule" TEXT
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'synonym_rule_is_valid_wrapper';

CREATE DOMAIN pdb.synonym_rule AS text CHECK (pdb.synonym_rule_is_valid(VALUE));

CREATE FUNCTION pdb."synonym_expansion"(
    "index" regclass,
    "input" TEXT
) RETURNS TABLE (
    "position" INT,
    "token" TEXT,
    "alternatives" TEXT[]
)
STRICT STABLE
LANGUAGE c
AS 'MODULE_PATHNAME', 'synonym_expansion_wrapper';

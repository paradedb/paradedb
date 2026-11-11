CREATE FUNCTION pdb."with_synonyms"(
    "query" TEXT,
    "index" regclass
) RETURNS pdb.Query
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'with_synonyms_wrapper';

CREATE FUNCTION pdb."with_synonyms"(
    "query" pdb.Query,
    "index" regclass
) RETURNS pdb.Query
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'with_synonyms_query_wrapper';

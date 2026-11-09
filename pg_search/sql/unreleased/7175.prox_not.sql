CREATE FUNCTION pdb."prox_not"(
    "include" pdb.ProximityClause,
    "exclude" pdb.ProximityClause,
    "pre" INT DEFAULT -1,
    "post" INT DEFAULT -1
) RETURNS pdb.ProximityClause
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'prox_not_wrapper';

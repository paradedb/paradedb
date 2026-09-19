-- Introduce the keyword spelling without changing existing tokenizer types or indexes.
CREATE TYPE pdb.keyword;
CREATE  FUNCTION pdb."keyword_in"(
	"s" cstring
) RETURNS pdb.keyword
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'keyword_in_wrapper';
CREATE  FUNCTION pdb."keyword_out"(
	"s" pdb.keyword
) RETURNS cstring
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'keyword_out_wrapper';
CREATE TYPE pdb.keyword (
    INPUT = pdb.keyword_in,
    OUTPUT = pdb.keyword_out,
    COLLATABLE = true,
    CATEGORY = 't',
    PREFERRED = false,
    INTERNALLENGTH = -1,
    ALIGNMENT = double,
    STORAGE = extended
);
ALTER TYPE pdb.keyword SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
CREATE  FUNCTION pdb."tokenize_keyword"(
	"s" pdb.keyword
) RETURNS TEXT[]
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'tokenize_keyword_wrapper';
CREATE CAST (pdb.keyword AS TEXT[]) WITH FUNCTION pdb.tokenize_keyword AS IMPLICIT;
CREATE CAST (text AS pdb.keyword) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.keyword) WITH INOUT AS IMPLICIT;
CREATE  FUNCTION pdb."json_to_keyword"(
	"json" json
) RETURNS pdb.keyword
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'json_to_keyword_wrapper';
CREATE CAST (json AS pdb.keyword) WITH FUNCTION pdb.json_to_keyword AS ASSIGNMENT;
CREATE  FUNCTION pdb."jsonb_to_keyword"(
	"jsonb" jsonb
) RETURNS pdb.keyword
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_to_keyword_wrapper';
CREATE CAST (jsonb AS pdb.keyword) WITH FUNCTION pdb.jsonb_to_keyword AS ASSIGNMENT;
CREATE  FUNCTION pdb."uuid_to_keyword"(
	"uuid" uuid
) RETURNS pdb.keyword
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_keyword_wrapper';
CREATE CAST (uuid AS pdb.keyword) WITH FUNCTION pdb.uuid_to_keyword AS ASSIGNMENT;
CREATE  FUNCTION pdb."text_array_to_keyword"(
	"arr" text[]
) RETURNS pdb.keyword
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'text_array_to_keyword_wrapper';
CREATE CAST (text[] AS pdb.keyword) WITH FUNCTION pdb.text_array_to_keyword AS ASSIGNMENT;
CREATE  FUNCTION pdb."varchar_array_to_keyword"(
	"arr" varchar[]
) RETURNS pdb.keyword
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'varchar_array_to_keyword_wrapper';
CREATE CAST (varchar[] AS pdb.keyword) WITH FUNCTION pdb.varchar_array_to_keyword AS ASSIGNMENT;
CREATE  FUNCTION pdb."keyword_to_boost"(
	"input" pdb.keyword,
	"typmod" INT,
	"is_explicit" bool
) RETURNS pdb.boost
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'keyword_to_boost_wrapper';
CREATE CAST (pdb.keyword AS pdb.boost) WITH FUNCTION pdb.keyword_to_boost(pdb.keyword, integer, boolean) AS ASSIGNMENT;
CREATE  FUNCTION pdb."keyword_to_const"(
	"input" pdb.keyword,
	"typmod" INT,
	"is_explicit" bool
) RETURNS pdb.const
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'keyword_to_const_wrapper';
CREATE CAST (pdb.keyword AS pdb.const) WITH FUNCTION pdb.keyword_to_const(pdb.keyword, integer, boolean) AS ASSIGNMENT;
CREATE  FUNCTION pdb."keyword_to_fuzzy"(
	"input" pdb.keyword,
	"typmod" INT,
	"is_explicit" bool
) RETURNS pdb.fuzzy
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'keyword_to_fuzzy_wrapper';
CREATE CAST (pdb.keyword AS pdb.fuzzy) WITH FUNCTION pdb.keyword_to_fuzzy(pdb.keyword, integer, boolean) AS ASSIGNMENT;
CREATE  FUNCTION pdb."keyword_to_slop"(
	"input" pdb.keyword,
	"typmod" INT,
	"is_explicit" bool
) RETURNS pdb.slop
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'keyword_to_slop_wrapper';
CREATE CAST (pdb.keyword AS pdb.slop) WITH FUNCTION pdb.keyword_to_slop(pdb.keyword, integer, boolean) AS ASSIGNMENT;

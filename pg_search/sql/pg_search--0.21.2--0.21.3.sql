DROP TYPE IF EXISTS pdb.icu CASCADE;
CREATE TYPE pdb.icu;
CREATE OR REPLACE FUNCTION pdb.icu_in(cstring) RETURNS pdb.icu AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.icu_out(pdb.icu) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.icu_send(pdb.icu) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.icu_recv(internal) RETURNS pdb.icu AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.icu (
                          INPUT = pdb.icu_in,
                          OUTPUT = pdb.icu_out,
                          SEND = pdb.icu_send,
                          RECEIVE = pdb.icu_recv,
                          COLLATABLE = true,
                          CATEGORY = 't', -- 't' is for tokenizer
                          PREFERRED = false,
                          LIKE = text
                      );

ALTER TYPE pdb.icu SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);

CREATE FUNCTION pdb."json_to_icu"(
    "json" json
) RETURNS pdb.icu
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'json_to_icu_wrapper';

CREATE FUNCTION pdb."jsonb_to_icu"(
    "jsonb" jsonb
) RETURNS pdb.icu
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_to_icu_wrapper';

CREATE FUNCTION pdb."tokenize_icu"(
    "s" pdb.icu
) RETURNS TEXT[]
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'tokenize_icu_wrapper';

CREATE FUNCTION pdb.varchar_array_to_icu(
    "arr" varchar[]
) RETURNS pdb.icu
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'varchar_array_to_icu_wrapper';

CREATE CAST (json AS pdb.icu) WITH FUNCTION pdb.json_to_icu AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.icu) WITH FUNCTION pdb.jsonb_to_icu AS ASSIGNMENT;
CREATE CAST (pdb.icu AS TEXT[]) WITH FUNCTION pdb.tokenize_icu AS IMPLICIT;
CREATE CAST (varchar[] AS pdb.icu) WITH FUNCTION pdb.varchar_array_to_icu AS ASSIGNMENT;

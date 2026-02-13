\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.21.9'" to load this file. \quit

CREATE TYPE pdb.normalized;
CREATE OR REPLACE FUNCTION pdb.normalized_in(cstring) RETURNS pdb.normalized AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.normalized_out(pdb.normalized) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.normalized_send(pdb.normalized) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.normalized_recv(internal) RETURNS pdb.normalized AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.normalized (
                INPUT = pdb.normalized_in,
                OUTPUT = pdb.normalized_out,
                SEND = pdb.normalized_send,
                RECEIVE = pdb.normalized_recv,
                INTERNALLENGTH = variable,
                STORAGE = extended,
                LIKE = text,
                CATEGORY = 't',
                COLLATABLE = true
            );
ALTER TYPE pdb.normalized SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);

CREATE FUNCTION pdb."tokenize_normalized"(
    "s" pdb.normalized
) RETURNS TEXT[] IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'tokenize_normalized_wrapper';

CREATE CAST (pdb.normalized AS TEXT[]) WITH FUNCTION pdb.tokenize_normalized AS IMPLICIT;

CREATE FUNCTION pdb."json_to_normalized"(
    "json" json
) RETURNS pdb.normalized IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'json_to_normalized_wrapper';

CREATE FUNCTION pdb."jsonb_to_normalized"(
    "jsonb" jsonb
) RETURNS pdb.normalized IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_to_normalized_wrapper';

CREATE FUNCTION pdb."uuid_to_normalized"(
    "uuid" uuid
) RETURNS pdb.normalized IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_normalized_wrapper';

CREATE FUNCTION pdb."text_array_to_normalized"(
    "arr" text[]
) RETURNS pdb.normalized IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'text_array_to_normalized_wrapper';

CREATE FUNCTION pdb."varchar_array_to_normalized"(
    "arr" varchar[]
) RETURNS pdb.normalized IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'varchar_array_to_normalized_wrapper';

CREATE CAST (json AS pdb.normalized) WITH FUNCTION pdb.json_to_normalized AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.normalized) WITH FUNCTION pdb.jsonb_to_normalized AS ASSIGNMENT;
CREATE CAST (uuid AS pdb.normalized) WITH FUNCTION pdb.uuid_to_normalized AS ASSIGNMENT;
CREATE CAST (text[] AS pdb.normalized) WITH FUNCTION pdb.text_array_to_normalized AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.normalized) WITH FUNCTION pdb.varchar_array_to_normalized AS ASSIGNMENT;
CREATE CAST (text AS pdb.normalized) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.normalized) WITH INOUT AS IMPLICIT;

CREATE OR REPLACE FUNCTION pdb."agg_placeholder_agg_placeholder_state"(
	"this" internal,
	"arg_one" jsonb
) RETURNS internal
LANGUAGE c
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_state_wrapper';

-- Ensure the single-arg finalize function exists
CREATE OR REPLACE FUNCTION pdb."agg_placeholder_agg_placeholder_finalize"(
	"this" internal
) RETURNS jsonb
LANGUAGE c
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_finalize_wrapper';

-- Recreate pdb.agg(jsonb) only if it doesn't already exist
DO $$
BEGIN
	IF NOT EXISTS (
		SELECT 1 FROM pg_aggregate a
		JOIN pg_proc p ON a.aggfnoid = p.oid
		JOIN pg_namespace n ON p.pronamespace = n.oid
		WHERE n.nspname = 'pdb' AND p.proname = 'agg' AND p.pronargs = 1
	) THEN
		CREATE AGGREGATE pdb.agg (jsonb) (
			SFUNC = pdb."agg_placeholder_agg_placeholder_state",
			STYPE = internal,
			FINALFUNC = pdb."agg_placeholder_agg_placeholder_finalize"
		);
	END IF;
END
$$;

-- Add edge_ngram tokenizer type
CREATE TYPE pdb.edge_ngram;
CREATE OR REPLACE FUNCTION pdb.edge_ngram_in(cstring) RETURNS pdb.edge_ngram AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.edge_ngram_out(pdb.edge_ngram) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.edge_ngram_send(pdb.edge_ngram) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.edge_ngram_recv(internal) RETURNS pdb.edge_ngram AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.edge_ngram (
    INPUT = pdb.edge_ngram_in,
    OUTPUT = pdb.edge_ngram_out,
    SEND = pdb.edge_ngram_send,
    RECEIVE = pdb.edge_ngram_recv,
    COLLATABLE = true,
    CATEGORY = 't',
    PREFERRED = false,
    LIKE = text
);

ALTER TYPE pdb.edge_ngram SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);

CREATE CAST (text AS pdb.edge_ngram) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.edge_ngram) WITH INOUT AS IMPLICIT;

CREATE FUNCTION pdb."tokenize_edge_ngram"(
    "s" pdb.edge_ngram
) RETURNS TEXT[]
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'tokenize_edge_ngram_wrapper';

CREATE CAST (pdb.edge_ngram AS TEXT[]) WITH FUNCTION pdb.tokenize_edge_ngram AS IMPLICIT;

CREATE FUNCTION pdb."json_to_edge_ngram"(
    "json" json
) RETURNS pdb.edge_ngram
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'json_to_edge_ngram_wrapper';

CREATE FUNCTION pdb."jsonb_to_edge_ngram"(
    "jsonb" jsonb
) RETURNS pdb.edge_ngram
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_to_edge_ngram_wrapper';

CREATE CAST (json AS pdb.edge_ngram) WITH FUNCTION pdb.json_to_edge_ngram AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.edge_ngram) WITH FUNCTION pdb.jsonb_to_edge_ngram AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_edge_ngram"(
    "uuid" uuid
) RETURNS pdb.edge_ngram
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_edge_ngram_wrapper';

CREATE CAST (uuid AS pdb.edge_ngram) WITH FUNCTION pdb.uuid_to_edge_ngram AS ASSIGNMENT;

CREATE FUNCTION pdb."text_array_to_edge_ngram"(
    "arr" text[]
) RETURNS pdb.edge_ngram
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'text_array_to_edge_ngram_wrapper';

CREATE FUNCTION pdb."varchar_array_to_edge_ngram"(
    "arr" varchar[]
) RETURNS pdb.edge_ngram
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c
AS 'MODULE_PATHNAME', 'varchar_array_to_edge_ngram_wrapper';

CREATE CAST (text[] AS pdb.edge_ngram) WITH FUNCTION pdb.text_array_to_edge_ngram AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.edge_ngram) WITH FUNCTION pdb.varchar_array_to_edge_ngram AS ASSIGNMENT;

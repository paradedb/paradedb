DROP FUNCTION IF EXISTS score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'paradedb_score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;

CREATE CAST (varchar AS paradedb.fieldname) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.ngram) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.ngram) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.whitespace) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.whitespace) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.chinese_compatible) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.chinese_compatible) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.lindera) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.lindera) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.literal) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.literal) WITH INOUT AS IMPLICIT;

ALTER TYPE pdb.literal SET (TYPMOD_IN = literal_typmod_in, TYPMOD_OUT = generic_typmod_out);

CREATE CAST (text AS pdb.literal_normalized) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.literal_normalized) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.jieba) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.jieba) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.source_code) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.source_code) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.simple) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.simple) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.regex_pattern) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.regex_pattern) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.icu) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.icu) WITH INOUT AS IMPLICIT;

CREATE CAST (text AS pdb.unicode_words) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.unicode_words) WITH INOUT AS IMPLICIT;

CREATE FUNCTION pdb."uuid_to_lindera"(
	"uuid" uuid
) RETURNS pdb.lindera
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_lindera_wrapper';

CREATE CAST (uuid AS pdb.lindera) WITH FUNCTION pdb.uuid_to_lindera AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_source_code"(
	"uuid" uuid
) RETURNS pdb.source_code
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_source_code_wrapper';

CREATE CAST (uuid AS pdb.source_code) WITH FUNCTION pdb.uuid_to_source_code AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_simple"(
	"uuid" uuid
) RETURNS pdb.simple
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_simple_wrapper';

CREATE CAST (uuid AS pdb.simple) WITH FUNCTION pdb.uuid_to_simple AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_whitespace"(
	"uuid" uuid
) RETURNS pdb.whitespace
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_whitespace_wrapper';

CREATE CAST (uuid AS pdb.whitespace) WITH FUNCTION pdb.uuid_to_whitespace AS ASSIGNMENT;

DROP FUNCTION IF EXISTS pdb.uuid_to_alias(arr uuid);
CREATE OR REPLACE FUNCTION pdb.uuid_to_alias(uuid uuid) RETURNS pdb.alias AS 'MODULE_PATHNAME', 'uuid_to_alias_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;

CREATE FUNCTION pdb."uuid_to_ngram"(
	"uuid" uuid
) RETURNS pdb.ngram
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_ngram_wrapper';

CREATE CAST (uuid AS pdb.ngram) WITH FUNCTION pdb.uuid_to_ngram AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_unicode_words"(
	"uuid" uuid
) RETURNS pdb.unicode_words
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_unicode_words_wrapper';

CREATE CAST (uuid AS pdb.unicode_words) WITH FUNCTION pdb.uuid_to_unicode_words AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_literal"(
	"uuid" uuid
) RETURNS pdb.literal
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_literal_wrapper';

CREATE CAST (uuid AS pdb.literal) WITH FUNCTION pdb.uuid_to_literal AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_literal_normalized"(
	"uuid" uuid
) RETURNS pdb.literal_normalized
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_literal_normalized_wrapper';

CREATE CAST (uuid AS pdb.literal_normalized) WITH FUNCTION pdb.uuid_to_literal_normalized AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_jieba"(
	"uuid" uuid
) RETURNS pdb.jieba
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_jieba_wrapper';

CREATE CAST (uuid AS pdb.jieba) WITH FUNCTION pdb.uuid_to_jieba AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_icu"(
	"uuid" uuid
) RETURNS pdb.icu
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_icu_wrapper';

CREATE CAST (uuid AS pdb.icu) WITH FUNCTION pdb.uuid_to_icu AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_chinese_compatible"(
	"uuid" uuid
) RETURNS pdb.chinese_compatible
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_chinese_compatible_wrapper';

CREATE CAST (uuid AS pdb.chinese_compatible) WITH FUNCTION pdb.uuid_to_chinese_compatible AS ASSIGNMENT;

CREATE FUNCTION pdb."uuid_to_regex_pattern"(
	"uuid" uuid
) RETURNS pdb.regex_pattern
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'uuid_to_regex_pattern_wrapper';

CREATE CAST (uuid AS pdb.regex_pattern) WITH FUNCTION pdb.uuid_to_regex_pattern AS ASSIGNMENT;

DROP FUNCTION IF EXISTS pdb.score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION pdb.score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;

CREATE FUNCTION pdb."indexes"() RETURNS TABLE (
	"schemaname" TEXT,
	"tablename" TEXT,
	"indexname" TEXT,
	"indexrelid" oid,
	"num_segments" INT,
	"total_docs" bigint
)
STRICT
LANGUAGE c
AS 'MODULE_PATHNAME', 'indexes_wrapper';

CREATE FUNCTION pdb."index_segments"(
	"index" regclass
) RETURNS TABLE (
	"partition_name" TEXT,
	"segment_idx" INT,
	"segment_id" TEXT,
	"num_docs" bigint,
	"num_deleted" bigint,
	"max_doc" bigint
)
STRICT
LANGUAGE c
AS 'MODULE_PATHNAME', 'index_segments_wrapper';

CREATE FUNCTION pdb."verify_index"(
	"index" regclass,
	"heapallindexed" bool DEFAULT false,
	"sample_rate" double precision DEFAULT NULL,
	"report_progress" bool DEFAULT false,
	"verbose" bool DEFAULT false,
	"on_error_stop" bool DEFAULT false,
	"segment_ids" INT[] DEFAULT NULL
) RETURNS TABLE (
	"check_name" TEXT,
	"passed" bool,
	"details" TEXT
)
LANGUAGE c
AS 'MODULE_PATHNAME', 'verify_index_wrapper';

CREATE FUNCTION pdb."verify_all_indexes"(
	"schema_pattern" TEXT DEFAULT NULL,
	"index_pattern" TEXT DEFAULT NULL,
	"heapallindexed" bool DEFAULT false,
	"sample_rate" double precision DEFAULT NULL,
	"report_progress" bool DEFAULT false,
	"on_error_stop" bool DEFAULT false
) RETURNS TABLE (
	"schemaname" TEXT,
	"indexname" TEXT,
	"check_name" TEXT,
	"passed" bool,
	"details" TEXT
)
LANGUAGE c
AS 'MODULE_PATHNAME', 'verify_all_indexes_wrapper';

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

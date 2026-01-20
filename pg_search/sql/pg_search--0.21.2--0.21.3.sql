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
DROP CAST IF EXISTS (uuid AS pdb.alias);

/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:414
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_simple
-- requires:
--   tokenize_simple
CREATE  FUNCTION pdb."uuid_to_simple"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.simple /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Simple, pg_search::api::tokenizers::definitions::pdb::SimpleUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_simple_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:414
-- requires:
--   simple_definition
--   uuid_to_simple

CREATE CAST (uuid AS pdb.simple) WITH FUNCTION pdb.uuid_to_simple AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:543
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_ngram
-- requires:
--   tokenize_ngram
CREATE  FUNCTION pdb."uuid_to_ngram"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.ngram /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Ngram, pg_search::api::tokenizers::definitions::pdb::NgramUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_ngram_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:543
-- requires:
--   ngram_definition
--   uuid_to_ngram

CREATE CAST (uuid AS pdb.ngram) WITH FUNCTION pdb.uuid_to_ngram AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:470
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_chinese_compatible
-- requires:
--   tokenize_chinese_compatible
CREATE  FUNCTION pdb."uuid_to_chinese_compatible"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.chinese_compatible /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::ChineseCompatible, pg_search::api::tokenizers::definitions::pdb::ChineseCompatibleUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_chinese_compatible_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:470
-- requires:
--   chinese_compatible_definition
--   uuid_to_chinese_compatible

CREATE CAST (uuid AS pdb.chinese_compatible) WITH FUNCTION pdb.uuid_to_chinese_compatible AS ASSIGNMENT;
DROP FUNCTION IF EXISTS pdb.uuid_to_alias(arr uuid);
CREATE OR REPLACE FUNCTION pdb.uuid_to_alias(uuid uuid) RETURNS pdb.alias AS 'MODULE_PATHNAME', 'uuid_to_alias_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:515
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_source_code
-- requires:
--   tokenize_source_code
CREATE  FUNCTION pdb."uuid_to_source_code"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.source_code /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::SourceCode, pg_search::api::tokenizers::definitions::pdb::SourceCodeUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_source_code_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:515
-- requires:
--   source_code_definition
--   uuid_to_source_code

CREATE CAST (uuid AS pdb.source_code) WITH FUNCTION pdb.uuid_to_source_code AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:562
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_regex_pattern
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."uuid_to_regex_pattern"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.regex_pattern /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Regex, pg_search::api::tokenizers::definitions::pdb::RegexUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_regex_pattern_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:562
-- requires:
--   regex_pattern_definition
--   uuid_to_regex_pattern

CREATE CAST (uuid AS pdb.regex_pattern) WITH FUNCTION pdb.uuid_to_regex_pattern AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:579
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_unicode_words
-- requires:
--   tokenize_unicode_words
CREATE  FUNCTION pdb."uuid_to_unicode_words"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.unicode_words /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::UnicodeWords, pg_search::api::tokenizers::definitions::pdb::UnicodeWordsUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_unicode_words_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:579
-- requires:
--   unicode_words_definition
--   uuid_to_unicode_words

CREATE CAST (uuid AS pdb.unicode_words) WITH FUNCTION pdb.uuid_to_unicode_words AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:484
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_lindera
-- requires:
--   tokenize_lindera
CREATE  FUNCTION pdb."uuid_to_lindera"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.lindera /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Lindera, pg_search::api::tokenizers::definitions::pdb::LinderaUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_lindera_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:484
-- requires:
--   lindera_definition
--   uuid_to_lindera

CREATE CAST (uuid AS pdb.lindera) WITH FUNCTION pdb.uuid_to_lindera AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:456
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_literal_normalized
-- requires:
--   tokenize_literal_normalized
CREATE  FUNCTION pdb."uuid_to_literal_normalized"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.literal_normalized /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::LiteralNormalized, pg_search::api::tokenizers::definitions::pdb::LiteralNormalizedUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_literal_normalized_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:456
-- requires:
--   literal_normalized_definition
--   uuid_to_literal_normalized

CREATE CAST (uuid AS pdb.literal_normalized) WITH FUNCTION pdb.uuid_to_literal_normalized AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:442
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_literal
-- requires:
--   tokenize_literal
CREATE  FUNCTION pdb."uuid_to_literal"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.literal /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Literal, pg_search::api::tokenizers::definitions::pdb::LiteralUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_literal_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:442
-- requires:
--   literal_definition
--   uuid_to_literal

CREATE CAST (uuid AS pdb.literal) WITH FUNCTION pdb.uuid_to_literal AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:498
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_jieba
-- requires:
--   tokenize_jieba
CREATE  FUNCTION pdb."uuid_to_jieba"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.jieba /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Jieba, pg_search::api::tokenizers::definitions::pdb::JiebaUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_jieba_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:498
-- requires:
--   jieba_definition
--   uuid_to_jieba

CREATE CAST (uuid AS pdb.jieba) WITH FUNCTION pdb.uuid_to_jieba AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:428
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_whitespace
-- requires:
--   tokenize_whitespace
CREATE  FUNCTION pdb."uuid_to_whitespace"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.whitespace /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Whitespace, pg_search::api::tokenizers::definitions::pdb::WhitespaceUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_whitespace_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:428
-- requires:
--   whitespace_definition
--   uuid_to_whitespace

CREATE CAST (uuid AS pdb.whitespace) WITH FUNCTION pdb.uuid_to_whitespace AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:529
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::Icu)

/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:529
-- pg_search::api::tokenizers::definitions::pdb::uuid_to_icu
-- requires:
--   tokenize_icu
CREATE  FUNCTION pdb."uuid_to_icu"(
	"uuid" uuid /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::uuid::Uuid, pg_search::api::tokenizers::UuidMarker> */
) RETURNS pdb.icu /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Icu, pg_search::api::tokenizers::definitions::pdb::IcuUuidMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_to_icu_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:529
-- requires:
--   icu_definition
--   uuid_to_icu

CREATE CAST (uuid AS pdb.icu) WITH FUNCTION pdb.uuid_to_icu AS ASSIGNMENT;

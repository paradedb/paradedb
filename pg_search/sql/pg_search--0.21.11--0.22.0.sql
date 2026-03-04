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

CREATE CAST (text AS pdb.simple) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.simple) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.whitespace) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.whitespace) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.literal) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.literal) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.literal_normalized) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.literal_normalized) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.chinese_compatible) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.chinese_compatible) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.lindera) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.lindera) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.jieba) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.jieba) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.source_code) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.source_code) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.icu) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.icu) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.ngram) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.ngram) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.regex_pattern) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.regex_pattern) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.unicode_words) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.unicode_words) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS paradedb.fieldname) WITH INOUT AS IMPLICIT;

DROP FUNCTION IF EXISTS score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'paradedb_score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;
DROP FUNCTION IF EXISTS pdb.score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION pdb.score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;

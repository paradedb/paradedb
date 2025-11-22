/* pg_search::api::aggregate::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::agg_placeholder_agg_placeholder_state
CREATE  FUNCTION pdb."agg_placeholder_agg_placeholder_state"(
	"this" internal, /* pgrx::datum::internal::Internal */
	"arg_one" jsonb /* pgrx::datum::json::JsonB */
) RETURNS internal /* pgrx::datum::internal::Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_state_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::agg_placeholder_agg_placeholder_finalize
CREATE  FUNCTION pdb."agg_placeholder_agg_placeholder_finalize"(
	"this" internal /* pgrx::datum::internal::Internal */
) RETURNS jsonb /* pgrx::datum::json::JsonB */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_finalize_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::AggPlaceholder
CREATE AGGREGATE pdb.agg (
	jsonb /* pgrx::datum::json::JsonB */
)
(
	SFUNC = pdb."agg_placeholder_agg_placeholder_state", /* pg_search::api::aggregate::pdb::AggPlaceholder::state */
	STYPE = internal, /* pgrx::datum::internal::Internal */
	FINALFUNC = pdb."agg_placeholder_agg_placeholder_finalize" /* pg_search::api::aggregate::pdb::AggPlaceholder::final */
);
/* pg_search::api::window_aggregate::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/window_aggregate.rs:57
-- pg_search::api::window_aggregate::pdb::window_agg
CREATE  FUNCTION pdb."window_agg"(
	"window_aggregate_json" TEXT /* &str */
) RETURNS bigint /* i64 */
STRICT VOLATILE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'window_agg_placeholder_wrapper';
/* pg_search::api::aggregate::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:150
-- pg_search::api::aggregate::pdb::agg_fn
CREATE  FUNCTION pdb."agg_fn"(
	"_agg_name" TEXT /* &str */
) RETURNS jsonb /* pgrx::datum::json::JsonB */
STRICT VOLATILE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_fn_placeholder_wrapper';

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:261
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_lindera
-- requires:
--   tokenize_lindera
CREATE  FUNCTION pdb."text_array_to_lindera"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.lindera /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Lindera> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_lindera_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:261
-- requires:
--   lindera_definition
--   text_array_to_lindera
CREATE CAST (text[] AS pdb.lindera) WITH FUNCTION pdb.text_array_to_lindera AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:201
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_simple
-- requires:
--   tokenize_simple
CREATE  FUNCTION pdb."text_array_to_simple"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.simple /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Simple> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_simple_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:201
-- requires:
--   simple_definition
--   text_array_to_simple
CREATE CAST (text[] AS pdb.simple) WITH FUNCTION pdb.text_array_to_simple AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:327
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_regex_pattern
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."text_array_to_regex_pattern"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.regex_pattern /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Regex> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_regex_pattern_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:327
-- requires:
--   regex_pattern_definition
--   text_array_to_regex_pattern
CREATE CAST (text[] AS pdb.regex_pattern) WITH FUNCTION pdb.text_array_to_regex_pattern AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:249
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_chinese_compatible
-- requires:
--   tokenize_chinese_compatible
CREATE  FUNCTION pdb."text_array_to_chinese_compatible"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.chinese_compatible /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::ChineseCompatible> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_chinese_compatible_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:249
-- requires:
--   chinese_compatible_definition
--   text_array_to_chinese_compatible
CREATE CAST (text[] AS pdb.chinese_compatible) WITH FUNCTION pdb.text_array_to_chinese_compatible AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:310
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_ngram
-- requires:
--   tokenize_ngram
CREATE  FUNCTION pdb."text_array_to_ngram"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.ngram /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Ngram> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_ngram_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:310
-- requires:
--   ngram_definition
--   text_array_to_ngram
CREATE CAST (text[] AS pdb.ngram) WITH FUNCTION pdb.text_array_to_ngram AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_literal
-- requires:
--   tokenize_literal
CREATE  FUNCTION pdb."text_array_to_literal"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.literal /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Literal> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_literal_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- requires:
--   literal_definition
--   text_array_to_literal
CREATE CAST (text[] AS pdb.literal) WITH FUNCTION pdb.text_array_to_literal AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:189
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."text_array_to_alias"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:189
-- requires:
--   alias_definition
--   text_array_to_alias
CREATE CAST (text[] AS pdb.alias) WITH FUNCTION pdb.text_array_to_alias AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:273
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_jieba
-- requires:
--   tokenize_jieba
CREATE  FUNCTION pdb."text_array_to_jieba"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.jieba /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Jieba> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_jieba_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:273
-- requires:
--   jieba_definition
--   text_array_to_jieba
CREATE CAST (text[] AS pdb.jieba) WITH FUNCTION pdb.text_array_to_jieba AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:285
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_source_code
-- requires:
--   tokenize_source_code
CREATE  FUNCTION pdb."text_array_to_source_code"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.source_code /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::SourceCode> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_source_code_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:285
-- requires:
--   source_code_definition
--   text_array_to_source_code
CREATE CAST (text[] AS pdb.source_code) WITH FUNCTION pdb.text_array_to_source_code AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:342
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_unicode_words
-- requires:
--   tokenize_unicode_words
CREATE  FUNCTION pdb."text_array_to_unicode_words"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.unicode_words /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::UnicodeWords> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_unicode_words_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:342
-- requires:
--   unicode_words_definition
--   text_array_to_unicode_words
CREATE CAST (text[] AS pdb.unicode_words) WITH FUNCTION pdb.text_array_to_unicode_words AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:237
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_literal_normalized
-- requires:
--   tokenize_literal_normalized
CREATE  FUNCTION pdb."text_array_to_literal_normalized"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.literal_normalized /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::LiteralNormalized> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_literal_normalized_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:237
-- requires:
--   literal_normalized_definition
--   text_array_to_literal_normalized
CREATE CAST (text[] AS pdb.literal_normalized) WITH FUNCTION pdb.text_array_to_literal_normalized AS ASSIGNMENT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:213
-- pg_search::api::tokenizers::definitions::pdb::text_array_to_whitespace
-- requires:
--   tokenize_whitespace
CREATE  FUNCTION pdb."text_array_to_whitespace"(
	"arr" text[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>> */
) RETURNS pdb.whitespace /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Whitespace> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_whitespace_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:213
-- requires:
--   whitespace_definition
--   text_array_to_whitespace
CREATE CAST (text[] AS pdb.whitespace) WITH FUNCTION pdb.text_array_to_whitespace AS ASSIGNMENT;

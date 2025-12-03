/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_lindera
-- requires:
--   tokenize_lindera
CREATE  FUNCTION pdb."varchar_array_to_lindera"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.lindera /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Lindera, pg_search::api::tokenizers::definitions::pdb::LinderaVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_lindera_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:232
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_simple
-- requires:
--   tokenize_simple
CREATE  FUNCTION pdb."varchar_array_to_simple"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.simple /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Simple, pg_search::api::tokenizers::definitions::pdb::SimpleVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_simple_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:384
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_unicode_words
-- requires:
--   tokenize_unicode_words
CREATE  FUNCTION pdb."varchar_array_to_unicode_words"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.unicode_words /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::UnicodeWords, pg_search::api::tokenizers::definitions::pdb::UnicodeWordsVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_unicode_words_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:258
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_literal
-- requires:
--   tokenize_literal
CREATE  FUNCTION pdb."varchar_array_to_literal"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.literal /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Literal, pg_search::api::tokenizers::definitions::pdb::LiteralVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_literal_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:368
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_regex_pattern
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."varchar_array_to_regex_pattern"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.regex_pattern /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Regex, pg_search::api::tokenizers::definitions::pdb::RegexVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_regex_pattern_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:323
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_source_code
-- requires:
--   tokenize_source_code
CREATE  FUNCTION pdb."varchar_array_to_source_code"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.source_code /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::SourceCode, pg_search::api::tokenizers::definitions::pdb::SourceCodeVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_source_code_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:271
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_literal_normalized
-- requires:
--   tokenize_literal_normalized
CREATE  FUNCTION pdb."varchar_array_to_literal_normalized"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.literal_normalized /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::LiteralNormalized, pg_search::api::tokenizers::definitions::pdb::LiteralNormalizedVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_literal_normalized_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:350
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_ngram
-- requires:
--   tokenize_ngram
CREATE  FUNCTION pdb."varchar_array_to_ngram"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.ngram /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Ngram, pg_search::api::tokenizers::definitions::pdb::NgramVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_ngram_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:219
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."varchar_array_to_alias"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias, pg_search::api::tokenizers::definitions::pdb::AliasVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_alias_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:245
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_whitespace
-- requires:
--   tokenize_whitespace
CREATE  FUNCTION pdb."varchar_array_to_whitespace"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.whitespace /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Whitespace, pg_search::api::tokenizers::definitions::pdb::WhitespaceVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_whitespace_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:284
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_chinese_compatible
-- requires:
--   tokenize_chinese_compatible
CREATE  FUNCTION pdb."varchar_array_to_chinese_compatible"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.chinese_compatible /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::ChineseCompatible, pg_search::api::tokenizers::definitions::pdb::ChineseCompatibleVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_chinese_compatible_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:310
-- pg_search::api::tokenizers::definitions::pdb::varchar_array_to_jieba
-- requires:
--   tokenize_jieba
CREATE  FUNCTION pdb."varchar_array_to_jieba"(
	"arr" varchar[] /* pg_search::api::tokenizers::GenericTypeWrapper<alloc::vec::Vec<alloc::string::String>, pg_search::api::tokenizers::VarcharArrayMarker> */
) RETURNS pdb.jieba /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Jieba, pg_search::api::tokenizers::definitions::pdb::JiebaVarcharArrayMarker> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'varchar_array_to_jieba_wrapper';

CREATE CAST (varchar[] AS pdb.regex_pattern) WITH FUNCTION pdb.varchar_array_to_regex_pattern AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.lindera) WITH FUNCTION pdb.varchar_array_to_lindera AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.simple) WITH FUNCTION pdb.varchar_array_to_simple AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.literal) WITH FUNCTION pdb.varchar_array_to_literal AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.unicode_words) WITH FUNCTION pdb.varchar_array_to_unicode_words AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.source_code) WITH FUNCTION pdb.varchar_array_to_source_code AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.literal_normalized) WITH FUNCTION pdb.varchar_array_to_literal_normalized AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.ngram) WITH FUNCTION pdb.varchar_array_to_ngram AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.chinese_compatible) WITH FUNCTION pdb.varchar_array_to_chinese_compatible AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.alias) WITH FUNCTION pdb.varchar_array_to_alias AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.jieba) WITH FUNCTION pdb.varchar_array_to_jieba AS ASSIGNMENT;
CREATE CAST (varchar[] AS pdb.whitespace) WITH FUNCTION pdb.varchar_array_to_whitespace AS ASSIGNMENT;

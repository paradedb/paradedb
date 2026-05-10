ALTER TYPE pdb.ngram SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.ngram_send(pdb.ngram);
DROP FUNCTION IF EXISTS pdb.ngram_recv(internal);

ALTER TYPE pdb.chinese_compatible SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.chinese_compatible_send(pdb.chinese_compatible);
DROP FUNCTION IF EXISTS pdb.chinese_compatible_recv(internal);

ALTER TYPE pdb.regex_pattern SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.regex_pattern_send(pdb.regex_pattern);
DROP FUNCTION IF EXISTS pdb.regex_pattern_recv(internal);

ALTER TYPE pdb.source_code SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.source_code_send(pdb.source_code);
DROP FUNCTION IF EXISTS pdb.source_code_recv(internal);

ALTER TYPE pdb.icu SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.icu_send(pdb.icu);
DROP FUNCTION IF EXISTS pdb.icu_recv(internal);

ALTER TYPE pdb.alias SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.alias_send(pdb.alias);
DROP FUNCTION IF EXISTS pdb.alias_recv(internal);

ALTER TYPE pdb.unicode_words SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.unicode_words_send(pdb.unicode_words);
DROP FUNCTION IF EXISTS pdb.unicode_words_recv(internal);

ALTER TYPE pdb.lindera SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.lindera_send(pdb.lindera);
DROP FUNCTION IF EXISTS pdb.lindera_recv(internal);

ALTER TYPE pdb.literal SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.literal_send(pdb.literal);
DROP FUNCTION IF EXISTS pdb.literal_recv(internal);

ALTER TYPE pdb.simple SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.simple_send(pdb.simple);
DROP FUNCTION IF EXISTS pdb.simple_recv(internal);

ALTER TYPE pdb.literal_normalized SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.literal_normalized_send(pdb.literal_normalized);
DROP FUNCTION IF EXISTS pdb.literal_normalized_recv(internal);

ALTER TYPE pdb.jieba SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.jieba_send(pdb.jieba);
DROP FUNCTION IF EXISTS pdb.jieba_recv(internal);

ALTER TYPE pdb.edge_ngram SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.edge_ngram_send(pdb.edge_ngram);
DROP FUNCTION IF EXISTS pdb.edge_ngram_recv(internal);

ALTER TYPE pdb.whitespace SET (RECEIVE = NONE, SEND = NONE, STORAGE = EXTENDED);
DROP FUNCTION IF EXISTS pdb.whitespace_send(pdb.whitespace);
DROP FUNCTION IF EXISTS pdb.whitespace_recv(internal);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:647
-- pg_search::api::tokenizers::definitions::pdb::chinese_compatible_in
-- requires:
--   ChineseCompatibleDef
CREATE OR REPLACE FUNCTION pdb."chinese_compatible_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.chinese_compatible /* GenericTypeWrapper < ChineseCompatible, ChineseCompatibleTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'chinese_compatible_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:750
-- pg_search::api::tokenizers::definitions::pdb::edge_ngram_in
-- requires:
--   EdgeNgramDef
CREATE OR REPLACE FUNCTION pdb."edge_ngram_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.edge_ngram /* GenericTypeWrapper < EdgeNgram, EdgeNgramTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'edge_ngram_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:647
-- pg_search::api::tokenizers::definitions::pdb::chinese_compatible_out
-- requires:
--   ChineseCompatibleDef
CREATE OR REPLACE FUNCTION pdb."chinese_compatible_out"(
	"s" pdb.chinese_compatible /* ChineseCompatible */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'chinese_compatible_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:662
-- pg_search::api::tokenizers::definitions::pdb::lindera_in
-- requires:
--   LinderaDef
CREATE OR REPLACE FUNCTION pdb."lindera_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.lindera /* GenericTypeWrapper < Lindera, LinderaTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'lindera_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:662
-- pg_search::api::tokenizers::definitions::pdb::lindera_out
-- requires:
--   LinderaDef
CREATE OR REPLACE FUNCTION pdb."lindera_out"(
	"s" pdb.lindera /* Lindera */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'lindera_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:770
-- pg_search::api::tokenizers::definitions::pdb::regex_pattern_in
-- requires:
--   RegexDef
CREATE OR REPLACE FUNCTION pdb."regex_pattern_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.regex_pattern /* GenericTypeWrapper < Regex, RegexTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_pattern_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:729
-- pg_search::api::tokenizers::definitions::pdb::ngram_in
-- requires:
--   NgramDef
CREATE OR REPLACE FUNCTION pdb."ngram_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.ngram /* GenericTypeWrapper < Ngram, NgramTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'ngram_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:729
-- pg_search::api::tokenizers::definitions::pdb::ngram_out
-- requires:
--   NgramDef
CREATE OR REPLACE FUNCTION pdb."ngram_out"(
	"s" pdb.ngram /* Ngram */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'ngram_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:770
-- pg_search::api::tokenizers::definitions::pdb::regex_pattern_out
-- requires:
--   RegexDef
CREATE OR REPLACE FUNCTION pdb."regex_pattern_out"(
	"s" pdb.regex_pattern /* Regex */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_pattern_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:617
-- pg_search::api::tokenizers::definitions::pdb::literal_in
-- requires:
--   LiteralDef
CREATE OR REPLACE FUNCTION pdb."literal_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.literal /* GenericTypeWrapper < Literal, LiteralTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:617
-- pg_search::api::tokenizers::definitions::pdb::literal_out
-- requires:
--   LiteralDef
CREATE OR REPLACE FUNCTION pdb."literal_out"(
	"s" pdb.literal /* Literal */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:532
-- pg_search::api::tokenizers::definitions::pdb::alias_in
-- requires:
--   AliasDef
CREATE OR REPLACE FUNCTION pdb."alias_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.alias /* GenericTypeWrapper < Alias, AliasTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'alias_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:632
-- pg_search::api::tokenizers::definitions::pdb::literal_normalized_in
-- requires:
--   LiteralNormalizedDef
CREATE OR REPLACE FUNCTION pdb."literal_normalized_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.literal_normalized /* GenericTypeWrapper < LiteralNormalized, LiteralNormalizedTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_normalized_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:587
-- pg_search::api::tokenizers::definitions::pdb::simple_in
-- requires:
--   SimpleDef
CREATE OR REPLACE FUNCTION pdb."simple_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.simple /* GenericTypeWrapper < Simple, SimpleTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'simple_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:602
-- pg_search::api::tokenizers::definitions::pdb::whitespace_in
-- requires:
--   WhitespaceDef
CREATE OR REPLACE FUNCTION pdb."whitespace_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.whitespace /* GenericTypeWrapper < Whitespace, WhitespaceTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'whitespace_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:602
-- pg_search::api::tokenizers::definitions::pdb::whitespace_out
-- requires:
--   WhitespaceDef
CREATE OR REPLACE FUNCTION pdb."whitespace_out"(
	"s" pdb.whitespace /* Whitespace */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'whitespace_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:632
-- pg_search::api::tokenizers::definitions::pdb::literal_normalized_out
-- requires:
--   LiteralNormalizedDef
CREATE OR REPLACE FUNCTION pdb."literal_normalized_out"(
	"s" pdb.literal_normalized /* LiteralNormalized */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_normalized_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:750
-- pg_search::api::tokenizers::definitions::pdb::edge_ngram_out
-- requires:
--   EdgeNgramDef
CREATE OR REPLACE FUNCTION pdb."edge_ngram_out"(
	"s" pdb.edge_ngram /* EdgeNgram */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'edge_ngram_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:788
-- pg_search::api::tokenizers::definitions::pdb::unicode_words_in
-- requires:
--   UnicodeWordsDef
CREATE OR REPLACE FUNCTION pdb."unicode_words_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.unicode_words /* GenericTypeWrapper < UnicodeWords, UnicodeWordsTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'unicode_words_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:788
-- pg_search::api::tokenizers::definitions::pdb::unicode_words_out
-- requires:
--   UnicodeWordsDef
CREATE OR REPLACE FUNCTION pdb."unicode_words_out"(
	"s" pdb.unicode_words /* UnicodeWords */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'unicode_words_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:532
-- pg_search::api::tokenizers::definitions::pdb::alias_out
-- requires:
--   AliasDef
CREATE OR REPLACE FUNCTION pdb."alias_out"(
	"s" pdb.alias /* Alias */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'alias_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:681
-- pg_search::api::tokenizers::definitions::pdb::jieba_in
-- requires:
--   JiebaDef
CREATE OR REPLACE FUNCTION pdb."jieba_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.jieba /* GenericTypeWrapper < Jieba, JiebaTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jieba_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:681
-- pg_search::api::tokenizers::definitions::pdb::jieba_out
-- requires:
--   JiebaDef
CREATE OR REPLACE FUNCTION pdb."jieba_out"(
	"s" pdb.jieba /* Jieba */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jieba_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:699
-- pg_search::api::tokenizers::definitions::pdb::source_code_in
-- requires:
--   SourceCodeDef
CREATE OR REPLACE FUNCTION pdb."source_code_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.source_code /* GenericTypeWrapper < SourceCode, SourceCodeTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'source_code_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:699
-- pg_search::api::tokenizers::definitions::pdb::source_code_out
-- requires:
--   SourceCodeDef
CREATE OR REPLACE FUNCTION pdb."source_code_out"(
	"s" pdb.source_code /* SourceCode */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'source_code_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:714
-- pg_search::api::tokenizers::definitions::pdb::icu_in
-- requires:
--   IcuDef
CREATE OR REPLACE FUNCTION pdb."icu_in"(
	"s" cstring /* & std :: ffi :: CStr */
) RETURNS pdb.icu /* GenericTypeWrapper < Icu, IcuTextMarker > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'icu_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:714
-- pg_search::api::tokenizers::definitions::pdb::icu_out
-- requires:
--   IcuDef
CREATE OR REPLACE FUNCTION pdb."icu_out"(
	"s" pdb.icu /* Icu */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'icu_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:587
-- pg_search::api::tokenizers::definitions::pdb::simple_out
-- requires:
--   SimpleDef
CREATE OR REPLACE FUNCTION pdb."simple_out"(
	"s" pdb.simple /* Simple */
) RETURNS cstring /* & '_ std :: ffi :: CStr */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'simple_out_wrapper';

DROP FUNCTION IF EXISTS pdb."alias_out_safe"(pdb.alias);


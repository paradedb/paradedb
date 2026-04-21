-- pg_search/src/api/operator/fuzzy.rs:241
-- pg_search::api::operator::fuzzy::fuzzy_to_const
CREATE  FUNCTION "fuzzy_to_const"(
	"input" pdb.fuzzy, /* pg_search::api::operator::fuzzy::FuzzyType */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'fuzzy_to_const_wrapper';

-- pg_search/src/api/operator/slop.rs:213
-- pg_search::api::operator::slop::slop_to_const
CREATE  FUNCTION "slop_to_const"(
	"input" pdb.slop, /* pg_search::api::operator::slop::SlopType */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'slop_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:496
-- pg_search::api::tokenizers::definitions::pdb::chinese_compatible_to_fuzzy
-- requires:
--   tokenize_chinese_compatible
CREATE  FUNCTION pdb."chinese_compatible_to_fuzzy"(
	"input" pdb.chinese_compatible, /* pg_search::api::tokenizers::definitions::pdb::ChineseCompatible */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'chinese_compatible_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:496
-- requires:
--   chinese_compatible_definition
--   tokenize_chinese_compatible
--   chinese_compatible_to_fuzzy
CREATE CAST (pdb.chinese_compatible AS pdb.fuzzy) WITH FUNCTION pdb.chinese_compatible_to_fuzzy(pdb.chinese_compatible, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:482
-- pg_search::api::tokenizers::definitions::pdb::literal_normalized_to_fuzzy
-- requires:
--   tokenize_literal_normalized
CREATE  FUNCTION pdb."literal_normalized_to_fuzzy"(
	"input" pdb.literal_normalized, /* pg_search::api::tokenizers::definitions::pdb::LiteralNormalized */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_normalized_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:482
-- requires:
--   literal_normalized_definition
--   tokenize_literal_normalized
--   literal_normalized_to_fuzzy
CREATE CAST (pdb.literal_normalized AS pdb.fuzzy) WITH FUNCTION pdb.literal_normalized_to_fuzzy(pdb.literal_normalized, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:482
-- pg_search::api::tokenizers::definitions::pdb::literal_normalized_to_const
-- requires:
--   tokenize_literal_normalized
CREATE  FUNCTION pdb."literal_normalized_to_const"(
	"input" pdb.literal_normalized, /* pg_search::api::tokenizers::definitions::pdb::LiteralNormalized */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_normalized_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:482
-- requires:
--   literal_normalized_definition
--   tokenize_literal_normalized
--   literal_normalized_to_const
CREATE CAST (pdb.literal_normalized AS pdb.const) WITH FUNCTION pdb.literal_normalized_to_const(pdb.literal_normalized, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:482
-- pg_search::api::tokenizers::definitions::pdb::literal_normalized_to_slop
-- requires:
--   tokenize_literal_normalized
CREATE  FUNCTION pdb."literal_normalized_to_slop"(
	"input" pdb.literal_normalized, /* pg_search::api::tokenizers::definitions::pdb::LiteralNormalized */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_normalized_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:482
-- requires:
--   literal_normalized_definition
--   tokenize_literal_normalized
--   literal_normalized_to_slop
CREATE CAST (pdb.literal_normalized AS pdb.slop) WITH FUNCTION pdb.literal_normalized_to_slop(pdb.literal_normalized, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:559
-- pg_search::api::tokenizers::definitions::pdb::icu_to_slop
-- requires:
--   tokenize_icu
CREATE  FUNCTION pdb."icu_to_slop"(
	"input" pdb.icu, /* pg_search::api::tokenizers::definitions::pdb::Icu */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'icu_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:559
-- requires:
--   icu_definition
--   tokenize_icu
--   icu_to_slop
CREATE CAST (pdb.icu AS pdb.slop) WITH FUNCTION pdb.icu_to_slop(pdb.icu, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:559
-- pg_search::api::tokenizers::definitions::pdb::icu_to_const
-- requires:
--   tokenize_icu
CREATE  FUNCTION pdb."icu_to_const"(
	"input" pdb.icu, /* pg_search::api::tokenizers::definitions::pdb::Icu */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'icu_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:559
-- requires:
--   icu_definition
--   tokenize_icu
--   icu_to_const
CREATE CAST (pdb.icu AS pdb.const) WITH FUNCTION pdb.icu_to_const(pdb.icu, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:528
-- pg_search::api::tokenizers::definitions::pdb::jieba_to_boost
-- requires:
--   tokenize_jieba
CREATE  FUNCTION pdb."jieba_to_boost"(
	"input" pdb.jieba, /* pg_search::api::tokenizers::definitions::pdb::Jieba */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jieba_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:528
-- requires:
--   jieba_definition
--   tokenize_jieba
--   jieba_to_boost
CREATE CAST (pdb.jieba AS pdb.boost) WITH FUNCTION pdb.jieba_to_boost(pdb.jieba, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:528
-- pg_search::api::tokenizers::definitions::pdb::jieba_to_const
-- requires:
--   tokenize_jieba
CREATE  FUNCTION pdb."jieba_to_const"(
	"input" pdb.jieba, /* pg_search::api::tokenizers::definitions::pdb::Jieba */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jieba_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:528
-- requires:
--   jieba_definition
--   tokenize_jieba
--   jieba_to_const
CREATE CAST (pdb.jieba AS pdb.const) WITH FUNCTION pdb.jieba_to_const(pdb.jieba, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:612
-- pg_search::api::tokenizers::definitions::pdb::regex_pattern_to_boost
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."regex_pattern_to_boost"(
	"input" pdb.regex_pattern, /* pg_search::api::tokenizers::definitions::pdb::Regex */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_pattern_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:612
-- requires:
--   regex_pattern_definition
--   tokenize_regex
--   regex_pattern_to_boost
CREATE CAST (pdb.regex_pattern AS pdb.boost) WITH FUNCTION pdb.regex_pattern_to_boost(pdb.regex_pattern, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:496
-- pg_search::api::tokenizers::definitions::pdb::chinese_compatible_to_slop
-- requires:
--   tokenize_chinese_compatible
CREATE  FUNCTION pdb."chinese_compatible_to_slop"(
	"input" pdb.chinese_compatible, /* pg_search::api::tokenizers::definitions::pdb::ChineseCompatible */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'chinese_compatible_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:496
-- requires:
--   chinese_compatible_definition
--   tokenize_chinese_compatible
--   chinese_compatible_to_slop
CREATE CAST (pdb.chinese_compatible AS pdb.slop) WITH FUNCTION pdb.chinese_compatible_to_slop(pdb.chinese_compatible, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:593
-- pg_search::api::tokenizers::definitions::pdb::edge_ngram_to_const
-- requires:
--   tokenize_edge_ngram
CREATE  FUNCTION pdb."edge_ngram_to_const"(
	"input" pdb.edge_ngram, /* pg_search::api::tokenizers::definitions::pdb::EdgeNgram */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'edge_ngram_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:593
-- requires:
--   edge_ngram_definition
--   tokenize_edge_ngram
--   edge_ngram_to_const
CREATE CAST (pdb.edge_ngram AS pdb.const) WITH FUNCTION pdb.edge_ngram_to_const(pdb.edge_ngram, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:375
-- pg_search::api::tokenizers::definitions::pdb::alias_to_slop
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."alias_to_slop"(
	"input" pdb.alias, /* pg_search::api::tokenizers::definitions::pdb::Alias */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'alias_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:375
-- requires:
--   alias_definition
--   tokenize_alias
--   alias_to_slop
CREATE CAST (pdb.alias AS pdb.slop) WITH FUNCTION pdb.alias_to_slop(pdb.alias, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:375
-- pg_search::api::tokenizers::definitions::pdb::alias_to_fuzzy
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."alias_to_fuzzy"(
	"input" pdb.alias, /* pg_search::api::tokenizers::definitions::pdb::Alias */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'alias_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:375
-- requires:
--   alias_definition
--   tokenize_alias
--   alias_to_fuzzy
CREATE CAST (pdb.alias AS pdb.fuzzy) WITH FUNCTION pdb.alias_to_fuzzy(pdb.alias, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:468
-- pg_search::api::tokenizers::definitions::pdb::literal_to_fuzzy
-- requires:
--   tokenize_literal
CREATE  FUNCTION pdb."literal_to_fuzzy"(
	"input" pdb.literal, /* pg_search::api::tokenizers::definitions::pdb::Literal */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:468
-- requires:
--   literal_definition
--   tokenize_literal
--   literal_to_fuzzy
CREATE CAST (pdb.literal AS pdb.fuzzy) WITH FUNCTION pdb.literal_to_fuzzy(pdb.literal, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:573
-- pg_search::api::tokenizers::definitions::pdb::ngram_to_slop
-- requires:
--   tokenize_ngram
CREATE  FUNCTION pdb."ngram_to_slop"(
	"input" pdb.ngram, /* pg_search::api::tokenizers::definitions::pdb::Ngram */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'ngram_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:573
-- requires:
--   ngram_definition
--   tokenize_ngram
--   ngram_to_slop
CREATE CAST (pdb.ngram AS pdb.slop) WITH FUNCTION pdb.ngram_to_slop(pdb.ngram, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:573
-- pg_search::api::tokenizers::definitions::pdb::ngram_to_boost
-- requires:
--   tokenize_ngram
CREATE  FUNCTION pdb."ngram_to_boost"(
	"input" pdb.ngram, /* pg_search::api::tokenizers::definitions::pdb::Ngram */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'ngram_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:573
-- requires:
--   ngram_definition
--   tokenize_ngram
--   ngram_to_boost
CREATE CAST (pdb.ngram AS pdb.boost) WITH FUNCTION pdb.ngram_to_boost(pdb.ngram, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:440
-- pg_search::api::tokenizers::definitions::pdb::simple_to_boost
-- requires:
--   tokenize_simple
CREATE  FUNCTION pdb."simple_to_boost"(
	"input" pdb.simple, /* pg_search::api::tokenizers::definitions::pdb::Simple */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'simple_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:440
-- requires:
--   simple_definition
--   tokenize_simple
--   simple_to_boost
CREATE CAST (pdb.simple AS pdb.boost) WITH FUNCTION pdb.simple_to_boost(pdb.simple, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:440
-- pg_search::api::tokenizers::definitions::pdb::simple_to_fuzzy
-- requires:
--   tokenize_simple
CREATE  FUNCTION pdb."simple_to_fuzzy"(
	"input" pdb.simple, /* pg_search::api::tokenizers::definitions::pdb::Simple */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'simple_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:440
-- requires:
--   simple_definition
--   tokenize_simple
--   simple_to_fuzzy
CREATE CAST (pdb.simple AS pdb.fuzzy) WITH FUNCTION pdb.simple_to_fuzzy(pdb.simple, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:440
-- pg_search::api::tokenizers::definitions::pdb::simple_to_const
-- requires:
--   tokenize_simple
CREATE  FUNCTION pdb."simple_to_const"(
	"input" pdb.simple, /* pg_search::api::tokenizers::definitions::pdb::Simple */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'simple_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:440
-- requires:
--   simple_definition
--   tokenize_simple
--   simple_to_const
CREATE CAST (pdb.simple AS pdb.const) WITH FUNCTION pdb.simple_to_const(pdb.simple, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:375
-- pg_search::api::tokenizers::definitions::pdb::alias_to_const
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."alias_to_const"(
	"input" pdb.alias, /* pg_search::api::tokenizers::definitions::pdb::Alias */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'alias_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:375
-- requires:
--   alias_definition
--   tokenize_alias
--   alias_to_const
CREATE CAST (pdb.alias AS pdb.const) WITH FUNCTION pdb.alias_to_const(pdb.alias, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:510
-- pg_search::api::tokenizers::definitions::pdb::lindera_to_fuzzy
-- requires:
--   tokenize_lindera
CREATE  FUNCTION pdb."lindera_to_fuzzy"(
	"input" pdb.lindera, /* pg_search::api::tokenizers::definitions::pdb::Lindera */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'lindera_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:510
-- requires:
--   lindera_definition
--   tokenize_lindera
--   lindera_to_fuzzy
CREATE CAST (pdb.lindera AS pdb.fuzzy) WITH FUNCTION pdb.lindera_to_fuzzy(pdb.lindera, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:510
-- pg_search::api::tokenizers::definitions::pdb::lindera_to_slop
-- requires:
--   tokenize_lindera
CREATE  FUNCTION pdb."lindera_to_slop"(
	"input" pdb.lindera, /* pg_search::api::tokenizers::definitions::pdb::Lindera */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'lindera_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:510
-- requires:
--   lindera_definition
--   tokenize_lindera
--   lindera_to_slop
CREATE CAST (pdb.lindera AS pdb.slop) WITH FUNCTION pdb.lindera_to_slop(pdb.lindera, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:440
-- pg_search::api::tokenizers::definitions::pdb::simple_to_slop
-- requires:
--   tokenize_simple
CREATE  FUNCTION pdb."simple_to_slop"(
	"input" pdb.simple, /* pg_search::api::tokenizers::definitions::pdb::Simple */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'simple_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:440
-- requires:
--   simple_definition
--   tokenize_simple
--   simple_to_slop
CREATE CAST (pdb.simple AS pdb.slop) WITH FUNCTION pdb.simple_to_slop(pdb.simple, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:496
-- pg_search::api::tokenizers::definitions::pdb::chinese_compatible_to_const
-- requires:
--   tokenize_chinese_compatible
CREATE  FUNCTION pdb."chinese_compatible_to_const"(
	"input" pdb.chinese_compatible, /* pg_search::api::tokenizers::definitions::pdb::ChineseCompatible */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'chinese_compatible_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:496
-- requires:
--   chinese_compatible_definition
--   tokenize_chinese_compatible
--   chinese_compatible_to_const
CREATE CAST (pdb.chinese_compatible AS pdb.const) WITH FUNCTION pdb.chinese_compatible_to_const(pdb.chinese_compatible, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:559
-- pg_search::api::tokenizers::definitions::pdb::icu_to_fuzzy
-- requires:
--   tokenize_icu
CREATE  FUNCTION pdb."icu_to_fuzzy"(
	"input" pdb.icu, /* pg_search::api::tokenizers::definitions::pdb::Icu */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'icu_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:559
-- requires:
--   icu_definition
--   tokenize_icu
--   icu_to_fuzzy
CREATE CAST (pdb.icu AS pdb.fuzzy) WITH FUNCTION pdb.icu_to_fuzzy(pdb.icu, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:573
-- pg_search::api::tokenizers::definitions::pdb::ngram_to_fuzzy
-- requires:
--   tokenize_ngram
CREATE  FUNCTION pdb."ngram_to_fuzzy"(
	"input" pdb.ngram, /* pg_search::api::tokenizers::definitions::pdb::Ngram */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'ngram_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:573
-- requires:
--   ngram_definition
--   tokenize_ngram
--   ngram_to_fuzzy
CREATE CAST (pdb.ngram AS pdb.fuzzy) WITH FUNCTION pdb.ngram_to_fuzzy(pdb.ngram, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:573
-- pg_search::api::tokenizers::definitions::pdb::ngram_to_const
-- requires:
--   tokenize_ngram
CREATE  FUNCTION pdb."ngram_to_const"(
	"input" pdb.ngram, /* pg_search::api::tokenizers::definitions::pdb::Ngram */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'ngram_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:573
-- requires:
--   ngram_definition
--   tokenize_ngram
--   ngram_to_const
CREATE CAST (pdb.ngram AS pdb.const) WITH FUNCTION pdb.ngram_to_const(pdb.ngram, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:593
-- pg_search::api::tokenizers::definitions::pdb::edge_ngram_to_slop
-- requires:
--   tokenize_edge_ngram
CREATE  FUNCTION pdb."edge_ngram_to_slop"(
	"input" pdb.edge_ngram, /* pg_search::api::tokenizers::definitions::pdb::EdgeNgram */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'edge_ngram_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:593
-- requires:
--   edge_ngram_definition
--   tokenize_edge_ngram
--   edge_ngram_to_slop
CREATE CAST (pdb.edge_ngram AS pdb.slop) WITH FUNCTION pdb.edge_ngram_to_slop(pdb.edge_ngram, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:528
-- pg_search::api::tokenizers::definitions::pdb::jieba_to_fuzzy
-- requires:
--   tokenize_jieba
CREATE  FUNCTION pdb."jieba_to_fuzzy"(
	"input" pdb.jieba, /* pg_search::api::tokenizers::definitions::pdb::Jieba */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jieba_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:528
-- requires:
--   jieba_definition
--   tokenize_jieba
--   jieba_to_fuzzy
CREATE CAST (pdb.jieba AS pdb.fuzzy) WITH FUNCTION pdb.jieba_to_fuzzy(pdb.jieba, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:375
-- pg_search::api::tokenizers::definitions::pdb::alias_to_boost
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."alias_to_boost"(
	"input" pdb.alias, /* pg_search::api::tokenizers::definitions::pdb::Alias */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'alias_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:375
-- requires:
--   alias_definition
--   tokenize_alias
--   alias_to_boost
CREATE CAST (pdb.alias AS pdb.boost) WITH FUNCTION pdb.alias_to_boost(pdb.alias, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:468
-- pg_search::api::tokenizers::definitions::pdb::literal_to_slop
-- requires:
--   tokenize_literal
CREATE  FUNCTION pdb."literal_to_slop"(
	"input" pdb.literal, /* pg_search::api::tokenizers::definitions::pdb::Literal */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:468
-- requires:
--   literal_definition
--   tokenize_literal
--   literal_to_slop
CREATE CAST (pdb.literal AS pdb.slop) WITH FUNCTION pdb.literal_to_slop(pdb.literal, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:468
-- pg_search::api::tokenizers::definitions::pdb::literal_to_const
-- requires:
--   tokenize_literal
CREATE  FUNCTION pdb."literal_to_const"(
	"input" pdb.literal, /* pg_search::api::tokenizers::definitions::pdb::Literal */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:468
-- requires:
--   literal_definition
--   tokenize_literal
--   literal_to_const
CREATE CAST (pdb.literal AS pdb.const) WITH FUNCTION pdb.literal_to_const(pdb.literal, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:612
-- pg_search::api::tokenizers::definitions::pdb::regex_pattern_to_const
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."regex_pattern_to_const"(
	"input" pdb.regex_pattern, /* pg_search::api::tokenizers::definitions::pdb::Regex */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_pattern_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:612
-- requires:
--   regex_pattern_definition
--   tokenize_regex
--   regex_pattern_to_const
CREATE CAST (pdb.regex_pattern AS pdb.const) WITH FUNCTION pdb.regex_pattern_to_const(pdb.regex_pattern, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:545
-- pg_search::api::tokenizers::definitions::pdb::source_code_to_slop
-- requires:
--   tokenize_source_code
CREATE  FUNCTION pdb."source_code_to_slop"(
	"input" pdb.source_code, /* pg_search::api::tokenizers::definitions::pdb::SourceCode */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'source_code_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:545
-- requires:
--   source_code_definition
--   tokenize_source_code
--   source_code_to_slop
CREATE CAST (pdb.source_code AS pdb.slop) WITH FUNCTION pdb.source_code_to_slop(pdb.source_code, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:545
-- pg_search::api::tokenizers::definitions::pdb::source_code_to_fuzzy
-- requires:
--   tokenize_source_code
CREATE  FUNCTION pdb."source_code_to_fuzzy"(
	"input" pdb.source_code, /* pg_search::api::tokenizers::definitions::pdb::SourceCode */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'source_code_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:545
-- requires:
--   source_code_definition
--   tokenize_source_code
--   source_code_to_fuzzy
CREATE CAST (pdb.source_code AS pdb.fuzzy) WITH FUNCTION pdb.source_code_to_fuzzy(pdb.source_code, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:545
-- pg_search::api::tokenizers::definitions::pdb::source_code_to_boost
-- requires:
--   tokenize_source_code
CREATE  FUNCTION pdb."source_code_to_boost"(
	"input" pdb.source_code, /* pg_search::api::tokenizers::definitions::pdb::SourceCode */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'source_code_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:545
-- requires:
--   source_code_definition
--   tokenize_source_code
--   source_code_to_boost
CREATE CAST (pdb.source_code AS pdb.boost) WITH FUNCTION pdb.source_code_to_boost(pdb.source_code, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:468
-- pg_search::api::tokenizers::definitions::pdb::literal_to_boost
-- requires:
--   tokenize_literal
CREATE  FUNCTION pdb."literal_to_boost"(
	"input" pdb.literal, /* pg_search::api::tokenizers::definitions::pdb::Literal */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:468
-- requires:
--   literal_definition
--   tokenize_literal
--   literal_to_boost
CREATE CAST (pdb.literal AS pdb.boost) WITH FUNCTION pdb.literal_to_boost(pdb.literal, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:612
-- pg_search::api::tokenizers::definitions::pdb::regex_pattern_to_slop
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."regex_pattern_to_slop"(
	"input" pdb.regex_pattern, /* pg_search::api::tokenizers::definitions::pdb::Regex */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_pattern_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:612
-- requires:
--   regex_pattern_definition
--   tokenize_regex
--   regex_pattern_to_slop
CREATE CAST (pdb.regex_pattern AS pdb.slop) WITH FUNCTION pdb.regex_pattern_to_slop(pdb.regex_pattern, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:559
-- pg_search::api::tokenizers::definitions::pdb::icu_to_boost
-- requires:
--   tokenize_icu
CREATE  FUNCTION pdb."icu_to_boost"(
	"input" pdb.icu, /* pg_search::api::tokenizers::definitions::pdb::Icu */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'icu_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:559
-- requires:
--   icu_definition
--   tokenize_icu
--   icu_to_boost
CREATE CAST (pdb.icu AS pdb.boost) WITH FUNCTION pdb.icu_to_boost(pdb.icu, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:612
-- pg_search::api::tokenizers::definitions::pdb::regex_pattern_to_fuzzy
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."regex_pattern_to_fuzzy"(
	"input" pdb.regex_pattern, /* pg_search::api::tokenizers::definitions::pdb::Regex */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_pattern_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:612
-- requires:
--   regex_pattern_definition
--   tokenize_regex
--   regex_pattern_to_fuzzy
CREATE CAST (pdb.regex_pattern AS pdb.fuzzy) WITH FUNCTION pdb.regex_pattern_to_fuzzy(pdb.regex_pattern, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:593
-- pg_search::api::tokenizers::definitions::pdb::edge_ngram_to_boost
-- requires:
--   tokenize_edge_ngram
CREATE  FUNCTION pdb."edge_ngram_to_boost"(
	"input" pdb.edge_ngram, /* pg_search::api::tokenizers::definitions::pdb::EdgeNgram */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'edge_ngram_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:593
-- requires:
--   edge_ngram_definition
--   tokenize_edge_ngram
--   edge_ngram_to_boost
CREATE CAST (pdb.edge_ngram AS pdb.boost) WITH FUNCTION pdb.edge_ngram_to_boost(pdb.edge_ngram, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:528
-- pg_search::api::tokenizers::definitions::pdb::jieba_to_slop
-- requires:
--   tokenize_jieba
CREATE  FUNCTION pdb."jieba_to_slop"(
	"input" pdb.jieba, /* pg_search::api::tokenizers::definitions::pdb::Jieba */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jieba_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:528
-- requires:
--   jieba_definition
--   tokenize_jieba
--   jieba_to_slop
CREATE CAST (pdb.jieba AS pdb.slop) WITH FUNCTION pdb.jieba_to_slop(pdb.jieba, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:510
-- pg_search::api::tokenizers::definitions::pdb::lindera_to_boost
-- requires:
--   tokenize_lindera
CREATE  FUNCTION pdb."lindera_to_boost"(
	"input" pdb.lindera, /* pg_search::api::tokenizers::definitions::pdb::Lindera */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'lindera_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:510
-- requires:
--   lindera_definition
--   tokenize_lindera
--   lindera_to_boost
CREATE CAST (pdb.lindera AS pdb.boost) WITH FUNCTION pdb.lindera_to_boost(pdb.lindera, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:629
-- pg_search::api::tokenizers::definitions::pdb::unicode_words_to_fuzzy
-- requires:
--   tokenize_unicode_words
CREATE  FUNCTION pdb."unicode_words_to_fuzzy"(
	"input" pdb.unicode_words, /* pg_search::api::tokenizers::definitions::pdb::UnicodeWords */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'unicode_words_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:629
-- requires:
--   unicode_words_definition
--   tokenize_unicode_words
--   unicode_words_to_fuzzy
CREATE CAST (pdb.unicode_words AS pdb.fuzzy) WITH FUNCTION pdb.unicode_words_to_fuzzy(pdb.unicode_words, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:629
-- pg_search::api::tokenizers::definitions::pdb::unicode_words_to_slop
-- requires:
--   tokenize_unicode_words
CREATE  FUNCTION pdb."unicode_words_to_slop"(
	"input" pdb.unicode_words, /* pg_search::api::tokenizers::definitions::pdb::UnicodeWords */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'unicode_words_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:629
-- requires:
--   unicode_words_definition
--   tokenize_unicode_words
--   unicode_words_to_slop
CREATE CAST (pdb.unicode_words AS pdb.slop) WITH FUNCTION pdb.unicode_words_to_slop(pdb.unicode_words, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:629
-- pg_search::api::tokenizers::definitions::pdb::unicode_words_to_boost
-- requires:
--   tokenize_unicode_words
CREATE  FUNCTION pdb."unicode_words_to_boost"(
	"input" pdb.unicode_words, /* pg_search::api::tokenizers::definitions::pdb::UnicodeWords */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'unicode_words_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:629
-- requires:
--   unicode_words_definition
--   tokenize_unicode_words
--   unicode_words_to_boost
CREATE CAST (pdb.unicode_words AS pdb.boost) WITH FUNCTION pdb.unicode_words_to_boost(pdb.unicode_words, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:629
-- pg_search::api::tokenizers::definitions::pdb::unicode_words_to_const
-- requires:
--   tokenize_unicode_words
CREATE  FUNCTION pdb."unicode_words_to_const"(
	"input" pdb.unicode_words, /* pg_search::api::tokenizers::definitions::pdb::UnicodeWords */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'unicode_words_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:629
-- requires:
--   unicode_words_definition
--   tokenize_unicode_words
--   unicode_words_to_const
CREATE CAST (pdb.unicode_words AS pdb.const) WITH FUNCTION pdb.unicode_words_to_const(pdb.unicode_words, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:482
-- pg_search::api::tokenizers::definitions::pdb::literal_normalized_to_boost
-- requires:
--   tokenize_literal_normalized
CREATE  FUNCTION pdb."literal_normalized_to_boost"(
	"input" pdb.literal_normalized, /* pg_search::api::tokenizers::definitions::pdb::LiteralNormalized */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_normalized_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:482
-- requires:
--   literal_normalized_definition
--   tokenize_literal_normalized
--   literal_normalized_to_boost
CREATE CAST (pdb.literal_normalized AS pdb.boost) WITH FUNCTION pdb.literal_normalized_to_boost(pdb.literal_normalized, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:593
-- pg_search::api::tokenizers::definitions::pdb::edge_ngram_to_fuzzy
-- requires:
--   tokenize_edge_ngram
CREATE  FUNCTION pdb."edge_ngram_to_fuzzy"(
	"input" pdb.edge_ngram, /* pg_search::api::tokenizers::definitions::pdb::EdgeNgram */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'edge_ngram_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:593
-- requires:
--   edge_ngram_definition
--   tokenize_edge_ngram
--   edge_ngram_to_fuzzy
CREATE CAST (pdb.edge_ngram AS pdb.fuzzy) WITH FUNCTION pdb.edge_ngram_to_fuzzy(pdb.edge_ngram, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:496
-- pg_search::api::tokenizers::definitions::pdb::chinese_compatible_to_boost
-- requires:
--   tokenize_chinese_compatible
CREATE  FUNCTION pdb."chinese_compatible_to_boost"(
	"input" pdb.chinese_compatible, /* pg_search::api::tokenizers::definitions::pdb::ChineseCompatible */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'chinese_compatible_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:496
-- requires:
--   chinese_compatible_definition
--   tokenize_chinese_compatible
--   chinese_compatible_to_boost
CREATE CAST (pdb.chinese_compatible AS pdb.boost) WITH FUNCTION pdb.chinese_compatible_to_boost(pdb.chinese_compatible, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:545
-- pg_search::api::tokenizers::definitions::pdb::source_code_to_const
-- requires:
--   tokenize_source_code
CREATE  FUNCTION pdb."source_code_to_const"(
	"input" pdb.source_code, /* pg_search::api::tokenizers::definitions::pdb::SourceCode */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'source_code_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:545
-- requires:
--   source_code_definition
--   tokenize_source_code
--   source_code_to_const
CREATE CAST (pdb.source_code AS pdb.const) WITH FUNCTION pdb.source_code_to_const(pdb.source_code, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:454
-- pg_search::api::tokenizers::definitions::pdb::whitespace_to_slop
-- requires:
--   tokenize_whitespace
CREATE  FUNCTION pdb."whitespace_to_slop"(
	"input" pdb.whitespace, /* pg_search::api::tokenizers::definitions::pdb::Whitespace */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'whitespace_to_slop_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:454
-- requires:
--   whitespace_definition
--   tokenize_whitespace
--   whitespace_to_slop
CREATE CAST (pdb.whitespace AS pdb.slop) WITH FUNCTION pdb.whitespace_to_slop(pdb.whitespace, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:454
-- pg_search::api::tokenizers::definitions::pdb::whitespace_to_const
-- requires:
--   tokenize_whitespace
CREATE  FUNCTION pdb."whitespace_to_const"(
	"input" pdb.whitespace, /* pg_search::api::tokenizers::definitions::pdb::Whitespace */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'whitespace_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:454
-- requires:
--   whitespace_definition
--   tokenize_whitespace
--   whitespace_to_const
CREATE CAST (pdb.whitespace AS pdb.const) WITH FUNCTION pdb.whitespace_to_const(pdb.whitespace, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:454
-- pg_search::api::tokenizers::definitions::pdb::whitespace_to_boost
-- requires:
--   tokenize_whitespace
CREATE  FUNCTION pdb."whitespace_to_boost"(
	"input" pdb.whitespace, /* pg_search::api::tokenizers::definitions::pdb::Whitespace */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'whitespace_to_boost_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:454
-- requires:
--   whitespace_definition
--   tokenize_whitespace
--   whitespace_to_boost
CREATE CAST (pdb.whitespace AS pdb.boost) WITH FUNCTION pdb.whitespace_to_boost(pdb.whitespace, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:454
-- pg_search::api::tokenizers::definitions::pdb::whitespace_to_fuzzy
-- requires:
--   tokenize_whitespace
CREATE  FUNCTION pdb."whitespace_to_fuzzy"(
	"input" pdb.whitespace, /* pg_search::api::tokenizers::definitions::pdb::Whitespace */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'whitespace_to_fuzzy_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:454
-- requires:
--   whitespace_definition
--   tokenize_whitespace
--   whitespace_to_fuzzy
CREATE CAST (pdb.whitespace AS pdb.fuzzy) WITH FUNCTION pdb.whitespace_to_fuzzy(pdb.whitespace, integer, boolean) AS ASSIGNMENT;

-- pg_search/src/api/tokenizers/definitions.rs:510
-- pg_search::api::tokenizers::definitions::pdb::lindera_to_const
-- requires:
--   tokenize_lindera
CREATE  FUNCTION pdb."lindera_to_const"(
	"input" pdb.lindera, /* pg_search::api::tokenizers::definitions::pdb::Lindera */
	"typmod" INT, /* i32 */
	"is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
IMMUTABLE STRICT PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'lindera_to_const_wrapper';

-- pg_search/src/api/tokenizers/definitions.rs:510
-- requires:
--   lindera_definition
--   tokenize_lindera
--   lindera_to_const
CREATE CAST (pdb.lindera AS pdb.const) WITH FUNCTION pdb.lindera_to_const(pdb.lindera, integer, boolean) AS ASSIGNMENT;
CREATE CAST (pdb.slop AS pdb.const) WITH FUNCTION slop_to_const(pdb.slop, integer, boolean) AS IMPLICIT;
CREATE CAST (pdb.fuzzy AS pdb.const) WITH FUNCTION fuzzy_to_const(pdb.fuzzy, integer, boolean) AS IMPLICIT;

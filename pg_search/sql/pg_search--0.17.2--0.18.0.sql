/*
Don't know how to ALTER:
CREATE OPERATOR pg_catalog.@@@(procedure=search_with_text, leftarg=text, rightarg=text, restrict=text_restrict)
DefineStmt(DefineStmt { kind: OBJECT_OPERATOR, oldstyle: false, defnames: Some([Value(Value { string: Some("pg_catalog"), int: None, float: None, bit_string: None, null: None }), Value(Value { string: Some("@@@"), int: None, float: None, bit_string: None, null: None })]), args: None, definition: Some([DefElem(DefElem { defnamespace: None, defname: Some("procedure"), arg: Some(TypeName(TypeName { names: Some([Value(Value { string: Some("search_with_parse"), int: None, float: None, bit_string: None, null: None })]), typeOid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, arrayBounds: None })), defaction: DEFELEM_UNSPEC }), DefElem(DefElem { defnamespace: None, defname: Some("leftarg"), arg: Some(TypeName(TypeName { names: Some([Value(Value { string: Some("text"), int: None, float: None, bit_string: None, null: None })]), typeOid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, arrayBounds: None })), defaction: DEFELEM_UNSPEC }), DefElem(DefElem { defnamespace: None, defname: Some("rightarg"), arg: Some(TypeName(TypeName { names: Some([Value(Value { string: Some("text"), int: None, float: None, bit_string: None, null: None })]), typeOid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, arrayBounds: None })), defaction: DEFELEM_UNSPEC })]), if_not_exists: false, replace: false })
DefineStmt { kind: OBJECT_OPERATOR, oldstyle: false, defnames: Some([Value(Value { string: Some("pg_catalog"), int: None, float: None, bit_string: None, null: None }), Value(Value { string: Some("@@@"), int: None, float: None, bit_string: None, null: None })]), args: None, definition: Some([DefElem(DefElem { defnamespace: None, defname: Some("procedure"), arg: Some(TypeName(TypeName { names: Some([Value(Value { string: Some("search_with_text"), int: None, float: None, bit_string: None, null: None })]), typeOid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, arrayBounds: None })), defaction: DEFELEM_UNSPEC }), DefElem(DefElem { defnamespace: None, defname: Some("leftarg"), arg: Some(TypeName(TypeName { names: Some([Value(Value { string: Some("text"), int: None, float: None, bit_string: None, null: None })]), typeOid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, arrayBounds: None })), defaction: DEFELEM_UNSPEC }), DefElem(DefElem { defnamespace: None, defname: Some("rightarg"), arg: Some(TypeName(TypeName { names: Some([Value(Value { string: Some("text"), int: None, float: None, bit_string: None, null: None })]), typeOid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, arrayBounds: None })), defaction: DEFELEM_UNSPEC }), DefElem(DefElem { defnamespace: None, defname: Some("restrict"), arg: Some(TypeName(TypeName { names: Some([Value(Value { string: Some("text_restrict"), int: None, float: None, bit_string: None, null: None })]), typeOid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, arrayBounds: None })), defaction: DEFELEM_UNSPEC })]), if_not_exists: false, replace: false }
*/
-- DROP FUNCTION IF EXISTS search_with_text(_element text, query text);
-- DROP FUNCTION IF EXISTS text_restrict(planner_info internal, operator_oid oid, args internal, _var_relid pg_catalog.int4);
-- DROP FUNCTION IF EXISTS text_support(arg internal);
/* </end connected objects> */
/* <begin connected objects> */

--
-- take care of the renaming and other adjustments for our existing `@@@` operator
--
-- this is a manual implementation of what schemabot tried to do above but couldn't
--

-- the restrict function is not needed for the text operator
ALTER OPERATOR pg_catalog.@@@(anyelement, text) SET (RESTRICT = NONE);
DROP FUNCTION IF EXISTS text_restrict(planner_info internal, operator_oid oid, args internal, _var_relid pg_catalog.int4);

-- rename these functions
ALTER FUNCTION text_support RENAME TO atatat_support;
ALTER FUNCTION search_with_text RENAME TO search_with_parse;

-- we just renamed these functions and now we need to replace them to get the proper symbol name in the catalog
CREATE OR REPLACE FUNCTION "atatat_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'atatat_support_wrapper';

CREATE OR REPLACE FUNCTION "search_with_parse"(
    "_element" anyelement, /* pgrx::datum::anyelement::AnyElement */
    "query" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    SUPPORT atatat_support   /* make sure to set the SUPPORT function! */
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_parse_wrapper';
ALTER FUNCTION paradedb.search_with_parse SUPPORT paradedb.atatat_support;


--
-- machine-generated upgrade code
--

DROP FUNCTION IF EXISTS "exists"(field fieldname);
DROP FUNCTION IF EXISTS fuzzy_term(field fieldname, value text, distance pg_catalog.int4, transposition_cost_one bool, prefix bool);
DROP FUNCTION IF EXISTS match(field fieldname, value text, tokenizer jsonb, distance pg_catalog.int4, transposition_cost_one bool, prefix bool, conjunction_mode bool);
DROP FUNCTION IF EXISTS parse_with_field(field fieldname, query_string text, lenient bool, conjunction_mode bool);
DROP FUNCTION IF EXISTS phrase(field fieldname, phrases text[], slop pg_catalog.int4);
DROP FUNCTION IF EXISTS phrase_prefix(field fieldname, phrases text[], max_expansion pg_catalog.int4);
DROP FUNCTION IF EXISTS range(field fieldname, range daterange);
DROP FUNCTION IF EXISTS range(field fieldname, range int4range);
DROP FUNCTION IF EXISTS range(field fieldname, range int8range);
DROP FUNCTION IF EXISTS range(field fieldname, range numrange);
DROP FUNCTION IF EXISTS range(field fieldname, range tsrange);
DROP FUNCTION IF EXISTS range(field fieldname, range tstzrange);
DROP FUNCTION IF EXISTS range_term(field fieldname, term date);
DROP FUNCTION IF EXISTS range_term(field fieldname, term pg_catalog.float4);
DROP FUNCTION IF EXISTS range_term(field fieldname, term pg_catalog.float8);
DROP FUNCTION IF EXISTS range_term(field fieldname, term pg_catalog.int2);
DROP FUNCTION IF EXISTS range_term(field fieldname, term pg_catalog.int4);
DROP FUNCTION IF EXISTS range_term(field fieldname, term pg_catalog.int8);
DROP FUNCTION IF EXISTS range_term(field fieldname, term "char");
DROP FUNCTION IF EXISTS range_term(field fieldname, term pg_catalog."numeric");
DROP FUNCTION IF EXISTS range_term(field fieldname, range daterange, relation rangerelation);
DROP FUNCTION IF EXISTS range_term(field fieldname, range int4range, relation rangerelation);
DROP FUNCTION IF EXISTS range_term(field fieldname, range int8range, relation rangerelation);
DROP FUNCTION IF EXISTS range_term(field fieldname, range numrange, relation rangerelation);
DROP FUNCTION IF EXISTS range_term(field fieldname, range tsrange, relation rangerelation);
DROP FUNCTION IF EXISTS range_term(field fieldname, range tstzrange, relation rangerelation);
DROP FUNCTION IF EXISTS range_term(field fieldname, term pg_catalog."timestamp");
DROP FUNCTION IF EXISTS range_term(field fieldname, term pg_catalog.timestamptz);
DROP FUNCTION IF EXISTS regex(field fieldname, pattern text);
DROP FUNCTION IF EXISTS regex_phrase(field fieldname, regexes text[], slop pg_catalog.int4, max_expansions pg_catalog.int4);
DROP FUNCTION IF EXISTS search_with_text(_element anyelement, query text);
DROP FUNCTION IF EXISTS term(field fieldname, value date);
DROP FUNCTION IF EXISTS term(field fieldname, value inet);
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog."numeric");
DROP FUNCTION IF EXISTS term(field fieldname, value anyenum);
DROP FUNCTION IF EXISTS term(field fieldname, value bool);
DROP FUNCTION IF EXISTS term(field fieldname, value bytea);
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog.float4);
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog.float8);
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog.int2);
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog.int4);
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog.int8);
DROP FUNCTION IF EXISTS term(field fieldname, value "char");
DROP FUNCTION IF EXISTS term(field fieldname, value text);
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog."time");
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog.timetz);
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog."timestamp");
DROP FUNCTION IF EXISTS term(field fieldname, value pg_catalog.timestamptz);
DROP FUNCTION IF EXISTS term(field fieldname, value uuid);
DROP FUNCTION IF EXISTS text_restrict(planner_info internal, operator_oid oid, args internal, _var_relid pg_catalog.int4);
DROP FUNCTION IF EXISTS text_support(arg internal);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:66
-- pg_search::api::builder_fns::pdb::pdb::_faae45::exists
CREATE  FUNCTION "exists"(
    "field" FieldName /* pg_search::api::FieldName */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'exists_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:72
-- pg_search::api::builder_fns::pdb::pdb::_f5c3b7::fuzzy_term
CREATE  FUNCTION "fuzzy_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
    "distance" INT DEFAULT NULL, /* core::option::Option<i32> */
    "transposition_cost_one" bool DEFAULT NULL, /* core::option::Option<bool> */
    "prefix" bool DEFAULT NULL /* core::option::Option<bool> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'fuzzy_term_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:88
-- pg_search::api::builder_fns::pdb::pdb::_607293::match
CREATE  FUNCTION "match"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" TEXT, /* alloc::string::String */
    "tokenizer" jsonb DEFAULT NULL, /* core::option::Option<pgrx::datum::json::JsonB> */
    "distance" INT DEFAULT NULL, /* core::option::Option<i32> */
    "transposition_cost_one" bool DEFAULT NULL, /* core::option::Option<bool> */
    "prefix" bool DEFAULT NULL, /* core::option::Option<bool> */
    "conjunction_mode" bool DEFAULT NULL /* core::option::Option<bool> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_query_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:32
-- pg_search::api::builder_fns::pdb::pdb::_297f0b::match_conjunction
CREATE  FUNCTION "match_conjunction"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms_to_tokenize" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_conjunction_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:45
-- pg_search::api::builder_fns::pdb::pdb::_58c718::match_disjunction
CREATE  FUNCTION "match_disjunction"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms_to_tokenize" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_disjunction_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:108
-- pg_search::api::builder_fns::pdb::pdb::_e60b55::parse_with_field
CREATE  FUNCTION "parse_with_field"(
    "field" FieldName, /* pg_search::api::FieldName */
    "query_string" TEXT, /* alloc::string::String */
    "lenient" bool DEFAULT NULL, /* core::option::Option<bool> */
    "conjunction_mode" bool DEFAULT NULL /* core::option::Option<bool> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'parse_with_field_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:122
-- pg_search::api::builder_fns::pdb::pdb::_8b37cd::phrase
CREATE  FUNCTION "phrase"(
    "field" FieldName, /* pg_search::api::FieldName */
    "phrases" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "slop" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:60
-- pg_search::api::builder_fns::pdb::pdb::_a50749::phrase
CREATE  FUNCTION "phrase"(
    "field" FieldName, /* pg_search::api::FieldName */
    "phrase" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_string_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:131
-- pg_search::api::builder_fns::pdb::pdb::_116d5a::phrase_prefix
CREATE  FUNCTION "phrase_prefix"(
    "field" FieldName, /* pg_search::api::FieldName */
    "phrases" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "max_expansion" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_prefix_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:287
-- pg_search::api::builder_fns::pdb::pdb::_c31827::range
CREATE  FUNCTION "range"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" daterange /* pgrx::datum::range::Range<pgrx::datum::date::Date> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_date_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:143
-- pg_search::api::builder_fns::pdb::pdb::_0688dc::range
CREATE  FUNCTION "range"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" int4range /* pgrx::datum::range::Range<i32> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_i32_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:169
-- pg_search::api::builder_fns::pdb::pdb::_66098d::range
CREATE  FUNCTION "range"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" int8range /* pgrx::datum::range::Range<i64> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_i64_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:194
-- pg_search::api::builder_fns::pdb::pdb::_a904d2::range
CREATE  FUNCTION "range"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" numrange /* pgrx::datum::range::Range<pgrx::datum::numeric::AnyNumeric> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_numeric_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:288
-- pg_search::api::builder_fns::pdb::pdb::_3447a3::range
CREATE  FUNCTION "range"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" tsrange /* pgrx::datum::range::Range<pgrx::datum::time_stamp::Timestamp> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_timestamp_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:289
-- pg_search::api::builder_fns::pdb::pdb::_636d4e::range
CREATE  FUNCTION "range"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" tstzrange /* pgrx::datum::range::Range<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_timestamptz_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:506
-- pg_search::api::builder_fns::pdb::pdb::_3ef2e9::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" date /* pgrx::datum::date::Date */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_date_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:503
-- pg_search::api::builder_fns::pdb::pdb::_a60130::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" real /* f32 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_f32_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:504
-- pg_search::api::builder_fns::pdb::pdb::_170b6a::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" double precision /* f64 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_f64_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:500
-- pg_search::api::builder_fns::pdb::pdb::_219d72::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" smallint /* i16 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i16_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:501
-- pg_search::api::builder_fns::pdb::pdb::_34cc2e::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" INT /* i32 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i32_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:502
-- pg_search::api::builder_fns::pdb::pdb::_5afcf7::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" bigint /* i64 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i64_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:499
-- pg_search::api::builder_fns::pdb::pdb::_128b91::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" "char" /* i8 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i8_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:505
-- pg_search::api::builder_fns::pdb::pdb::_8e5cc2::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" NUMERIC /* pgrx::datum::numeric::AnyNumeric */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_numeric_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:617
-- pg_search::api::builder_fns::pdb::pdb::_d96989::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" daterange, /* pgrx::datum::range::Range<pgrx::datum::date::Date> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_daterange_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:599
-- pg_search::api::builder_fns::pdb::pdb::_5c9394::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" int4range, /* pgrx::datum::range::Range<i32> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_int4range_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:605
-- pg_search::api::builder_fns::pdb::pdb::_0b434f::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" int8range, /* pgrx::datum::range::Range<i64> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_int8range_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:611
-- pg_search::api::builder_fns::pdb::pdb::_e98f32::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" numrange, /* pgrx::datum::range::Range<pgrx::datum::numeric::AnyNumeric> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_numrange_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:623
-- pg_search::api::builder_fns::pdb::pdb::_76f962::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" tsrange, /* pgrx::datum::range::Range<pgrx::datum::time_stamp::Timestamp> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_tsrange_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:629
-- pg_search::api::builder_fns::pdb::pdb::_b19094::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "range" tstzrange, /* pgrx::datum::range::Range<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_tstzrange_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:507
-- pg_search::api::builder_fns::pdb::pdb::_d741f1::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" timestamp /* pgrx::datum::time_stamp::Timestamp */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_timestamp_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:508
-- pg_search::api::builder_fns::pdb::pdb::_d90b08::range_term
CREATE  FUNCTION "range_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" timestamp with time zone /* pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_timestamp_with_time_zone_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:372
-- pg_search::api::builder_fns::pdb::pdb::_2ed357::regex
CREATE  FUNCTION "regex"(
    "field" FieldName, /* pg_search::api::FieldName */
    "pattern" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:378
-- pg_search::api::builder_fns::pdb::pdb::_13d7d4::regex_phrase
CREATE  FUNCTION "regex_phrase"(
    "field" FieldName, /* pg_search::api::FieldName */
    "regexes" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "slop" INT DEFAULT NULL, /* core::option::Option<i32> */
    "max_expansions" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_phrase_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/andandand.rs:27
-- pg_search::api::operator::andandand::search_with_match_conjunction
CREATE  FUNCTION "search_with_match_conjunction"(
    "_field" TEXT, /* &str */
    "terms_to_tokenize" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_conjunction_wrapper';
-- pg_search/src/api/operator/andandand.rs:27
-- pg_search::api::operator::andandand::search_with_match_conjunction
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction",
    LEFTARG=TEXT, /* &str */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/andandand.rs:35
-- pg_search::api::operator::andandand::search_with_match_conjunction_support
CREATE  FUNCTION "search_with_match_conjunction_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_conjunction_support_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/andandand.rs:75
-- requires:
--   search_with_match_conjunction
--   search_with_match_conjunction_support

ALTER FUNCTION paradedb.search_with_match_conjunction SUPPORT paradedb.search_with_match_conjunction_support;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/ororor.rs:27
-- pg_search::api::operator::ororor::search_with_match_disjunction
CREATE  FUNCTION "search_with_match_disjunction"(
    "_field" TEXT, /* &str */
    "terms_to_tokenize" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_disjunction_wrapper';
-- pg_search/src/api/operator/ororor.rs:27
-- pg_search::api::operator::ororor::search_with_match_disjunction
CREATE OPERATOR pg_catalog.||| (
    PROCEDURE="search_with_match_disjunction",
    LEFTARG=TEXT, /* &str */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/ororor.rs:35
-- pg_search::api::operator::ororor::search_with_match_disjunction_support
CREATE  FUNCTION "search_with_match_disjunction_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_disjunction_support_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/ororor.rs:74
-- requires:
--   search_with_match_disjunction
--   search_with_match_disjunction_support

ALTER FUNCTION paradedb.search_with_match_disjunction SUPPORT paradedb.search_with_match_disjunction_support;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/hashhashhash.rs:27
-- pg_search::api::operator::hashhashhash::search_with_phrase
CREATE  FUNCTION "search_with_phrase"(
    "_field" TEXT, /* &str */
    "terms_to_tokenize" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_phrase_wrapper';
-- pg_search/src/api/operator/hashhashhash.rs:27
-- pg_search::api::operator::hashhashhash::search_with_phrase
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase",
    LEFTARG=TEXT, /* &str */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/hashhashhash.rs:35
-- pg_search::api::operator::hashhashhash::search_with_phrase_support
CREATE  FUNCTION "search_with_phrase_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_phrase_support_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/hashhashhash.rs:76
-- requires:
--   search_with_phrase
--   search_with_phrase_support

ALTER FUNCTION paradedb.search_with_phrase SUPPORT paradedb.search_with_phrase_support;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/eqeqeq.rs:27
-- pg_search::api::operator::eqeqeq::search_with_term
CREATE  FUNCTION "search_with_term"(
    "_field" TEXT, /* &str */
    "term" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_term_wrapper';
-- pg_search/src/api/operator/eqeqeq.rs:27
-- pg_search::api::operator::eqeqeq::search_with_term
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term",
    LEFTARG=TEXT, /* &str */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/eqeqeq.rs:33
-- pg_search::api::operator::eqeqeq::search_with_term_array
CREATE  FUNCTION "search_with_term_array"(
    "_field" TEXT, /* &str */
    "terms" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_term_array_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/eqeqeq.rs:39
-- pg_search::api::operator::eqeqeq::search_with_term_support
CREATE  FUNCTION "search_with_term_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_term_support_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/eqeqeq.rs:92
-- requires:
--   search_with_term
--   search_with_term_array
--   search_with_term_support


ALTER FUNCTION paradedb.search_with_term SUPPORT paradedb.search_with_term_support;
ALTER FUNCTION paradedb.search_with_term_array SUPPORT paradedb.search_with_term_support;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:435
-- pg_search::api::builder_fns::pdb::pdb::_8c3253::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" date /* pgrx::datum::date::Date */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'date_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:442
-- pg_search::api::builder_fns::pdb::pdb::_df33ca::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" inet /* pgrx::datum::inet::Inet */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'inet_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:440
-- pg_search::api::builder_fns::pdb::pdb::_5f0021::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" NUMERIC /* pgrx::datum::numeric::AnyNumeric */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'numeric_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:413
-- pg_search::api::builder_fns::pdb::pdb::_a1046b::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" anyenum /* pg_search::schema::anyenum::AnyEnum */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_anyenum_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:434
-- pg_search::api::builder_fns::pdb::pdb::_d75322::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" bool /* bool */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_bool_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:426
-- pg_search::api::builder_fns::pdb::pdb::_299ce8::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" bytea /* alloc::vec::Vec<u8> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_bytes_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:432
-- pg_search::api::builder_fns::pdb::pdb::_28a864::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" real /* f32 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_f32_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:433
-- pg_search::api::builder_fns::pdb::pdb::_fdf209::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" double precision /* f64 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_f64_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:429
-- pg_search::api::builder_fns::pdb::pdb::_3db7fc::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" smallint /* i16 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_i16_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:430
-- pg_search::api::builder_fns::pdb::pdb::_97cfaf::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" INT /* i32 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_i32_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:431
-- pg_search::api::builder_fns::pdb::pdb::_a0c8c6::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" bigint /* i64 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_i64_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:428
-- pg_search::api::builder_fns::pdb::pdb::_ee6048::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" "char" /* i8 */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_i8_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:427
-- pg_search::api::builder_fns::pdb::pdb::_cf633b::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_str_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:436
-- pg_search::api::builder_fns::pdb::pdb::_2f1cb2::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" time /* pgrx::datum::time::Time */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'time_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:438
-- pg_search::api::builder_fns::pdb::pdb::_f71566::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" time with time zone /* pgrx::datum::time_with_timezone::TimeWithTimeZone */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'time_with_time_zone_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:437
-- pg_search::api::builder_fns::pdb::pdb::_940494::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" timestamp /* pgrx::datum::time_stamp::Timestamp */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'timestamp_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:439
-- pg_search::api::builder_fns::pdb::pdb::_0caa34::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" timestamp with time zone /* pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'timestamp_with_time_zone_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:441
-- pg_search::api::builder_fns::pdb::pdb::_d5f4af::term
CREATE  FUNCTION "term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "value" uuid /* pgrx::datum::uuid::Uuid */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_bfn_wrapper';
DROP FUNCTION IF EXISTS term_set(terms searchqueryinput[]);
CREATE OR REPLACE FUNCTION term_set(terms searchqueryinput[]) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'term_set_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:469
-- pg_search::api::builder_fns::pdb::pdb::_7751c7::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" bool[] /* alloc::vec::Vec<bool> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_bool_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:470
-- pg_search::api::builder_fns::pdb::pdb::_ba3b82::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" date[] /* alloc::vec::Vec<pgrx::datum::date::Date> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_date_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:467
-- pg_search::api::builder_fns::pdb::pdb::_b7792c::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" real[] /* alloc::vec::Vec<f32> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_f32_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:468
-- pg_search::api::builder_fns::pdb::pdb::_2edd05::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" double precision[] /* alloc::vec::Vec<f64> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_f64_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:464
-- pg_search::api::builder_fns::pdb::pdb::_fcbdb1::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" smallint[] /* alloc::vec::Vec<i16> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_i16_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:465
-- pg_search::api::builder_fns::pdb::pdb::_08293a::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" INT[] /* alloc::vec::Vec<i32> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_i32_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:466
-- pg_search::api::builder_fns::pdb::pdb::_ba6e10::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" bigint[] /* alloc::vec::Vec<i64> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_i64_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:463
-- pg_search::api::builder_fns::pdb::pdb::_54a686::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" "char"[] /* alloc::vec::Vec<i8> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_i8_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:478
-- pg_search::api::builder_fns::pdb::pdb::_e15f16::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" NUMERIC[] /* alloc::vec::Vec<pgrx::datum::numeric::AnyNumeric> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_numeric_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:462
-- pg_search::api::builder_fns::pdb::pdb::_788317::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_str_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:471
-- pg_search::api::builder_fns::pdb::pdb::_5bb8ea::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" time[] /* alloc::vec::Vec<pgrx::datum::time::Time> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_time_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:473
-- pg_search::api::builder_fns::pdb::pdb::_33a6f3::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" time with time zone[] /* alloc::vec::Vec<pgrx::datum::time_with_timezone::TimeWithTimeZone> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_time_with_time_zone_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:472
-- pg_search::api::builder_fns::pdb::pdb::_a261f5::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" timestamp[] /* alloc::vec::Vec<pgrx::datum::time_stamp::Timestamp> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_timestamp_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:474
-- pg_search::api::builder_fns::pdb::pdb::_ffbf7c::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" timestamp with time zone[] /* alloc::vec::Vec<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_timestamp_with_time_zone_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:479
-- pg_search::api::builder_fns::pdb::pdb::_300bd9::term_set
CREATE  FUNCTION "term_set"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" uuid[] /* alloc::vec::Vec<pgrx::datum::uuid::Uuid> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_uuid_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/query/pdb_query.rs:25
CREATE SCHEMA IF NOT EXISTS pdb;
/* pg_search::query::pdb_query::pdb */
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/query/pdb_query.rs:33
-- pg_search::query::pdb_query::pdb::Query
CREATE TYPE pdb.Query;
-- pg_search/src/query/pdb_query.rs:33
-- pg_search::query::pdb_query::pdb::query_in
CREATE  FUNCTION pdb."query_in"(
    "input" cstring /* core::option::Option<&core::ffi::c_str::CStr> */
) RETURNS pdb.Query /* core::option::Option<pg_search::query::pdb_query::pdb::Query> */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'query_in_wrapper';
-- pg_search/src/query/pdb_query.rs:33
-- pg_search::query::pdb_query::pdb::query_out
CREATE  FUNCTION pdb."query_out"(
    "input" pdb.Query /* pg_search::query::pdb_query::pdb::Query */
) RETURNS cstring /* alloc::ffi::c_str::CString */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'query_out_wrapper';

-- pg_search/src/query/pdb_query.rs:33
-- pg_search::query::pdb_query::pdb::Query
CREATE TYPE pdb.Query (
  INTERNALLENGTH = variable,
  INPUT = pdb.query_in, /* pg_search::query::pdb_query::pdb::query_in */
  OUTPUT = pdb.query_out, /* pg_search::query::pdb_query::pdb::query_out */
  STORAGE = extended
);

/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/atatat.rs:39
-- pg_search::api::operator::atatat::search_with_fieled_query_input
CREATE  FUNCTION "search_with_fieled_query_input"(
    "_element" anyelement, /* pgrx::datum::anyelement::AnyElement */
    "query" pdb.Query /* pg_search::query::pdb_query::pdb::Query */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_fieled_query_input_wrapper';

-- pg_search/src/api/operator/atatat.rs:39
-- pg_search::api::operator::atatat::search_with_fieled_query_input
CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE="search_with_fieled_query_input",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    );

/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/atatat.rs:136
-- requires:
--   search_with_parse
--   search_with_fieled_query_input
--   atatat_support


ALTER FUNCTION paradedb.search_with_fieled_query_input SUPPORT paradedb.atatat_support;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/query/pdb_query.rs:20
-- pg_search::query::pdb_query::to_search_query_input
CREATE  FUNCTION "to_search_query_input"(
    "field" FieldName, /* pg_search::api::FieldName */
    "query" pdb.Query /* pg_search::query::pdb_query::pdb::Query */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'to_search_query_input_wrapper';
/* pg_search::api::builder_fns::pdb::pdb */
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:433
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" double precision /* f64 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_f64_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:471
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" time[] /* alloc::vec::Vec<pgrx::datum::time::Time> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_time_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:131
-- pg_search::api::builder_fns::pdb::pdb::phrase_prefix
CREATE  FUNCTION pdb."phrase_prefix"(
    "phrases" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "max_expansion" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_prefix_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:435
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" date /* pgrx::datum::date::Date */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'date_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:434
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" bool /* bool */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_bool_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:629
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "range" tstzrange, /* pgrx::datum::range::Range<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_tstzrange_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:439
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" timestamp with time zone /* pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'timestamp_with_time_zone_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:429
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" smallint /* i16 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_i16_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:507
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" timestamp /* pgrx::datum::time_stamp::Timestamp */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_timestamp_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:440
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" NUMERIC /* pgrx::datum::numeric::AnyNumeric */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'numeric_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:194
-- pg_search::api::builder_fns::pdb::pdb::range
CREATE  FUNCTION pdb."range"(
    "range" numrange /* pgrx::datum::range::Range<pgrx::datum::numeric::AnyNumeric> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_numeric_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:478
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" NUMERIC[] /* alloc::vec::Vec<pgrx::datum::numeric::AnyNumeric> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_numeric_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:502
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" bigint /* i64 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i64_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:436
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" time /* pgrx::datum::time::Time */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'time_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:506
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" date /* pgrx::datum::date::Date */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_date_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:599
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "range" int4range, /* pgrx::datum::range::Range<i32> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_int4range_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:469
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" bool[] /* alloc::vec::Vec<bool> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_bool_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:468
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" double precision[] /* alloc::vec::Vec<f64> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_f64_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:623
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "range" tsrange, /* pgrx::datum::range::Range<pgrx::datum::time_stamp::Timestamp> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_tsrange_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:378
-- pg_search::api::builder_fns::pdb::pdb::regex_phrase
CREATE  FUNCTION pdb."regex_phrase"(
    "regexes" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "slop" INT DEFAULT NULL, /* core::option::Option<i32> */
    "max_expansions" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_phrase_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:413
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" anyenum /* pg_search::schema::anyenum::AnyEnum */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_anyenum_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:464
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" smallint[] /* alloc::vec::Vec<i16> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_i16_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:470
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" date[] /* alloc::vec::Vec<pgrx::datum::date::Date> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_date_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:462
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_str_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:122
-- pg_search::api::builder_fns::pdb::pdb::phrase
CREATE  FUNCTION pdb."phrase"(
    "phrases" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "slop" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:441
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" uuid /* pgrx::datum::uuid::Uuid */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'uuid_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:499
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" "char" /* i8 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i8_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:426
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" bytea /* alloc::vec::Vec<u8> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_bytes_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:472
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" timestamp[] /* alloc::vec::Vec<pgrx::datum::time_stamp::Timestamp> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_timestamp_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:605
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "range" int8range, /* pgrx::datum::range::Range<i64> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_int8range_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:34
-- pg_search::api::builder_fns::pdb::pdb::match_conjunction
CREATE  FUNCTION pdb."match_conjunction"(
    "terms_to_tokenize" TEXT /* alloc::string::String */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_conjunction_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:108
-- pg_search::api::builder_fns::pdb::pdb::parse_with_field
CREATE  FUNCTION pdb."parse_with_field"(
    "query_string" TEXT, /* alloc::string::String */
    "lenient" bool DEFAULT NULL, /* core::option::Option<bool> */
    "conjunction_mode" bool DEFAULT NULL /* core::option::Option<bool> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'parse_with_field_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:503
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" real /* f32 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_f32_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:289
-- pg_search::api::builder_fns::pdb::pdb::range
CREATE  FUNCTION pdb."range"(
    "range" tstzrange /* pgrx::datum::range::Range<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_timestamptz_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:143
-- pg_search::api::builder_fns::pdb::pdb::range
CREATE  FUNCTION pdb."range"(
    "range" int4range /* pgrx::datum::range::Range<i32> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_i32_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:438
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" time with time zone /* pgrx::datum::time_with_timezone::TimeWithTimeZone */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'time_with_time_zone_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:473
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" time with time zone[] /* alloc::vec::Vec<pgrx::datum::time_with_timezone::TimeWithTimeZone> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_time_with_time_zone_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:467
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" real[] /* alloc::vec::Vec<f32> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_f32_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:287
-- pg_search::api::builder_fns::pdb::pdb::range
CREATE  FUNCTION pdb."range"(
    "range" daterange /* pgrx::datum::range::Range<pgrx::datum::date::Date> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_date_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:617
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "range" daterange, /* pgrx::datum::range::Range<pgrx::datum::date::Date> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_daterange_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:372
-- pg_search::api::builder_fns::pdb::pdb::regex
CREATE  FUNCTION pdb."regex"(
    "pattern" TEXT /* alloc::string::String */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'regex_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:466
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" bigint[] /* alloc::vec::Vec<i64> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_i64_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:60
-- pg_search::api::builder_fns::pdb::pdb::phrase
CREATE  FUNCTION pdb."phrase"(
    "phrase" TEXT /* alloc::string::String */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_string_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:66
-- pg_search::api::builder_fns::pdb::pdb::exists
CREATE  FUNCTION pdb."exists"() RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'exists_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:505
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" NUMERIC /* pgrx::datum::numeric::AnyNumeric */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_numeric_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:474
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" timestamp with time zone[] /* alloc::vec::Vec<pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_timestamp_with_time_zone_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:47
-- pg_search::api::builder_fns::pdb::pdb::match_disjunction
CREATE  FUNCTION pdb."match_disjunction"(
    "terms_to_tokenize" TEXT /* alloc::string::String */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_disjunction_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:430
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" INT /* i32 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_i32_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:501
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" INT /* i32 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i32_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:437
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" timestamp /* pgrx::datum::time_stamp::Timestamp */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'timestamp_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:288
-- pg_search::api::builder_fns::pdb::pdb::range
CREATE  FUNCTION pdb."range"(
    "range" tsrange /* pgrx::datum::range::Range<pgrx::datum::time_stamp::Timestamp> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_timestamp_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:500
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" smallint /* i16 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_i16_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:504
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" double precision /* f64 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_f64_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:611
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "range" numrange, /* pgrx::datum::range::Range<pgrx::datum::numeric::AnyNumeric> */
    "relation" RangeRelation /* pg_search::api::builder_fns::pdb::pdb::paradedb::RangeRelation */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_range_numrange_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:88
-- pg_search::api::builder_fns::pdb::pdb::match
CREATE  FUNCTION pdb."match"(
    "value" TEXT, /* alloc::string::String */
    "tokenizer" jsonb DEFAULT NULL, /* core::option::Option<pgrx::datum::json::JsonB> */
    "distance" INT DEFAULT NULL, /* core::option::Option<i32> */
    "transposition_cost_one" bool DEFAULT NULL, /* core::option::Option<bool> */
    "prefix" bool DEFAULT NULL, /* core::option::Option<bool> */
    "conjunction_mode" bool DEFAULT NULL /* core::option::Option<bool> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_query_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:508
-- pg_search::api::builder_fns::pdb::pdb::range_term
CREATE  FUNCTION pdb."range_term"(
    "term" timestamp with time zone /* pgrx::datum::time_stamp_with_timezone::TimestampWithTimeZone */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_term_timestamp_with_time_zone_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:432
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" real /* f32 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_f32_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:465
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" INT[] /* alloc::vec::Vec<i32> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_i32_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:463
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" "char"[] /* alloc::vec::Vec<i8> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_i8_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:442
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" inet /* pgrx::datum::inet::Inet */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'inet_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:72
-- pg_search::api::builder_fns::pdb::pdb::fuzzy_term
CREATE  FUNCTION pdb."fuzzy_term"(
    "value" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
    "distance" INT DEFAULT NULL, /* core::option::Option<i32> */
    "transposition_cost_one" bool DEFAULT NULL, /* core::option::Option<bool> */
    "prefix" bool DEFAULT NULL /* core::option::Option<bool> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'fuzzy_term_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:431
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" bigint /* i64 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_i64_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:427
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" TEXT /* alloc::string::String */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_str_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:428
-- pg_search::api::builder_fns::pdb::pdb::term
CREATE  FUNCTION pdb."term"(
    "value" "char" /* i8 */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_i8_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:169
-- pg_search::api::builder_fns::pdb::pdb::range
CREATE  FUNCTION pdb."range"(
    "range" int8range /* pgrx::datum::range::Range<i64> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'range_i64_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:479
-- pg_search::api::builder_fns::pdb::pdb::term_set
CREATE  FUNCTION pdb."term_set"(
    "terms" uuid[] /* alloc::vec::Vec<pgrx::datum::uuid::Uuid> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_uuid_wrapper';
DROP SCHEMA IF EXISTS paradedb_tmp;


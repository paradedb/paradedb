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
ALTER FUNCTION text_support RENAME TO search_with_parse_support;
ALTER FUNCTION search_with_text RENAME TO search_with_parse;

-- we just renamed these functions and now we need to replace them to get the proper symbol name in the catalog
CREATE OR REPLACE FUNCTION "search_with_parse_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_parse_support_wrapper';

CREATE OR REPLACE FUNCTION "search_with_parse"(
    "_element" anyelement, /* pgrx::datum::anyelement::AnyElement */
    "query" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    SUPPORT search_with_parse_support   /* make sure to set the SUPPORT function! */
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_parse_wrapper';
ALTER FUNCTION paradedb.search_with_parse SUPPORT paradedb.search_with_parse_support;


--
-- machine-generated upgrade code
--

/* <begin connected objects> */
-- pg_search/src/api/operator/andandand.rs:35
-- pg_search::api::operator::andandand::match_conjunction
CREATE  FUNCTION "match_conjunction"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms_to_tokenize" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_conjunction_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/ororor.rs:35
-- pg_search::api::operator::ororor::match_disjunction
CREATE  FUNCTION "match_disjunction"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms_to_tokenize" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_disjunction_wrapper';
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
-- pg_search/src/api/operator/andandand.rs:48
-- pg_search::api::operator::andandand::search_with_match_conjunction_support
CREATE  FUNCTION "search_with_match_conjunction_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_conjunction_support_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/andandand.rs:83
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
-- pg_search/src/api/operator/ororor.rs:48
-- pg_search::api::operator::ororor::search_with_match_disjunction_support
CREATE  FUNCTION "search_with_match_disjunction_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_disjunction_support_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/ororor.rs:84
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
-- pg_search/src/api/operator/hashhashhash.rs:44
-- pg_search::api::operator::hashhashhash::search_with_phrase_support
CREATE  FUNCTION "search_with_phrase_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_phrase_support_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/hashhashhash.rs:83
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
-- pg_search/src/api/operator/eqeqeq.rs:62
-- pg_search::api::operator::eqeqeq::search_with_term_support
CREATE  FUNCTION "search_with_term_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_term_support_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/eqeqeq.rs:114
-- requires:
--   search_with_term
--   search_with_term_array
--   search_with_term_support
    ALTER FUNCTION paradedb.search_with_term SUPPORT paradedb.search_with_term_support;
ALTER FUNCTION paradedb.search_with_term_array SUPPORT paradedb.search_with_term_support;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/eqeqeq.rs:39
-- pg_search::api::operator::eqeqeq::string_term
CREATE  FUNCTION "string_term"(
    "field" FieldName, /* pg_search::api::FieldName */
    "term" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'string_term_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/eqeqeq.rs:48
-- pg_search::api::operator::eqeqeq::string_term_array
CREATE  FUNCTION "string_term_array"(
    "field" FieldName, /* pg_search::api::FieldName */
    "terms" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'string_term_array_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/hashhashhash.rs:35
-- pg_search::api::operator::hashhashhash::tokenized_phrase
CREATE  FUNCTION "tokenized_phrase"(
    "field" FieldName, /* pg_search::api::FieldName */
    "phrase" TEXT /* alloc::string::String */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenized_phrase_wrapper';



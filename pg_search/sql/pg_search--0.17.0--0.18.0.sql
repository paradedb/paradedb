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
ALTER OPERATOR pg_catalog.@@@(text, text) SET (RESTRICT = NONE);
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
    "_element" text, /* &str */
    "query" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    SUPPORT search_with_parse_support   /* make sure to set the SUPPORT function! */
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_parse_wrapper';


--
-- machine-generated upgrade code
--

-- pg_search/src/api/operator/specialized/andandand.rs:21
-- pg_search::api::operator::specialized::andandand::search_with_match_conjunction
CREATE  FUNCTION "search_with_match_conjunction"(
    "_field" text, /* &str */
    "terms_to_tokenize" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_conjunction_wrapper';
-- pg_search/src/api/operator/specialized/andandand.rs:21
-- pg_search::api::operator::specialized::andandand::search_with_match_conjunction
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction",
    LEFTARG=text, /* &str */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/andandand.rs:27
-- pg_search::api::operator::specialized::andandand::search_with_match_conjunction_support
CREATE  FUNCTION "search_with_match_conjunction_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_conjunction_support_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/andandand.rs:45
-- requires:
--   search_with_match_conjunction
--   search_with_match_conjunction_support
    ALTER FUNCTION paradedb.search_with_match_conjunction SUPPORT paradedb.search_with_match_conjunction_support;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/ororor.rs:21
-- pg_search::api::operator::specialized::ororor::search_with_match_disjunction
CREATE  FUNCTION "search_with_match_disjunction"(
    "_field" text, /* &str */
    "terms_to_tokenize" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_disjunction_wrapper';
-- pg_search/src/api/operator/specialized/ororor.rs:21
-- pg_search::api::operator::specialized::ororor::search_with_match_disjunction
CREATE OPERATOR pg_catalog.||| (
    PROCEDURE="search_with_match_disjunction",
    LEFTARG=text, /* &str */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/ororor.rs:27
-- pg_search::api::operator::specialized::ororor::search_with_match_disjunction_support
CREATE  FUNCTION "search_with_match_disjunction_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_disjunction_support_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/ororor.rs:45
-- requires:
--   search_with_match_disjunction
--   search_with_match_disjunction_support
    ALTER FUNCTION paradedb.search_with_match_disjunction SUPPORT paradedb.search_with_match_disjunction_support;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/hashhashhash.rs:21
-- pg_search::api::operator::specialized::hashhashhash::search_with_phrase
CREATE  FUNCTION "search_with_phrase"(
    "_field" text, /* &str */
    "terms_to_tokenize" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_phrase_wrapper';
-- pg_search/src/api/operator/specialized/hashhashhash.rs:21
-- pg_search::api::operator::specialized::hashhashhash::search_with_phrase
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase",
    LEFTARG=text, /* &str */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/hashhashhash.rs:27
-- pg_search::api::operator::specialized::hashhashhash::search_with_phrase_support
CREATE  FUNCTION "search_with_phrase_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_phrase_support_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/hashhashhash.rs:41
-- requires:
--   search_with_phrase
--   search_with_phrase_support
ALTER FUNCTION paradedb.search_with_phrase SUPPORT paradedb.search_with_phrase_support;

--
-- ===(text)
--

/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/eqeqeq.rs:22
-- pg_search::api::operator::specialized::eqeqeq::search_with_term
CREATE  FUNCTION "search_with_term"(
    "_field" text, /* &str */
    "term" TEXT /* &str */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_term_wrapper';

-- pg_search/src/api/operator/specialized/eqeqeq.rs:22
-- pg_search::api::operator::specialized::eqeqeq::search_with_term
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term",
    LEFTARG=text, /* &str */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */

--
-- ===(text[])
--

/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/eqeqeq.rs:28
-- pg_search::api::operator::specialized::eqeqeq::search_with_term_array
CREATE  FUNCTION "search_with_term_array"(
    "_field" text, /* &str */
    "terms" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_term_array_wrapper';

-- pg_search/src/api/operator/specialized/eqeqeq.rs:28
-- pg_search::api::operator::specialized::eqeqeq::search_with_term_array
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term_array",
    LEFTARG=text, /* &str */
    RIGHTARG=TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/eqeqeq.rs:34
-- pg_search::api::operator::specialized::eqeqeq::search_with_term_support
CREATE  FUNCTION "search_with_term_support"(
    "arg" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pg_search::api::operator::ReturnedNodePointer */
    IMMUTABLE PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_term_support_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/specialized/eqeqeq.rs:70
-- requires:
--   search_with_term
--   search_with_term_array
--   search_with_term_support
ALTER FUNCTION paradedb.search_with_term SUPPORT paradedb.search_with_term_support;
ALTER FUNCTION paradedb.search_with_term_array SUPPORT paradedb.search_with_term_support;
/* </end connected objects> */


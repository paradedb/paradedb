-- pg_search/src/postgres/storage/metadata.rs:404
-- pg_search::postgres::storage::metadata::bgmerger_state
CREATE  FUNCTION "bgmerger_state"(
    "index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
                    "pid" INT,  /* i32 */
                    "state" TEXT  /* alloc::string::String */
                )
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'bgmerger_state_wrapper';

-- pg_search/src/postgres/storage/metadata.rs:397
-- pg_search::postgres::storage::metadata::reset_bgworker_state
CREATE  FUNCTION "reset_bgworker_state"(
    "index" regclass /* pgrx::rel::PgRelation */
) RETURNS void
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'reset_bgworker_state_wrapper';

-- pg_search/src/postgres/storage/fsm.rs:1358
-- pg_search::postgres::storage::fsm::fsm_size
CREATE  FUNCTION "fsm_size"(
    "index" regclass /* pgrx::rel::PgRelation */
) RETURNS bigint /* i64 */
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'fsm_size_wrapper';

DROP FUNCTION IF EXISTS fsm_info(index regclass);
CREATE OR REPLACE FUNCTION fsm_info(index regclass) RETURNS TABLE(xid xid, fsm_blockno pg_catalog.int8, tag pg_catalog.int8, free_blockno pg_catalog.int8) AS 'MODULE_PATHNAME', 'fsm_info_wrapper' LANGUAGE c STRICT;

--
-- new in-order proximity operator
--
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/proximity.rs:37
-- pg_search::api::operator::proximity::lhs_prox_in_order
CREATE  FUNCTION "lhs_prox_in_order"(
    "left" pdb.ProximityClause, /* pg_search::query::proximity::pdb::ProximityClause */
    "distance" INT /* i32 */
) RETURNS pdb.ProximityClause /* pg_search::query::proximity::pdb::ProximityClause */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'lhs_prox_in_order_wrapper';
-- pg_search/src/api/operator/proximity.rs:37
-- pg_search::api::operator::proximity::lhs_prox_in_order
CREATE OPERATOR pg_catalog.##> (
    PROCEDURE="lhs_prox_in_order",
    LEFTARG=pdb.ProximityClause, /* pg_search::query::proximity::pdb::ProximityClause */
    RIGHTARG=INT /* i32 */
    );
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/operator/proximity.rs:51
-- pg_search::api::operator::proximity::rhs_prox_in_order
CREATE  FUNCTION "rhs_prox_in_order"(
    "left" pdb.ProximityClause, /* pg_search::query::proximity::pdb::ProximityClause */
    "right" pdb.ProximityClause /* pg_search::query::proximity::pdb::ProximityClause */
) RETURNS pdb.ProximityClause /* pg_search::query::proximity::pdb::ProximityClause */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'rhs_prox_in_order_wrapper';
-- pg_search/src/api/operator/proximity.rs:51
-- pg_search::api::operator::proximity::rhs_prox_in_order
CREATE OPERATOR pg_catalog.##> (
    PROCEDURE="rhs_prox_in_order",
    LEFTARG=pdb.ProximityClause, /* pg_search::query::proximity::pdb::ProximityClause */
    RIGHTARG=pdb.ProximityClause /* pg_search::query::proximity::pdb::ProximityClause */
    );


--
-- relocate the fuzzy, boost, and slop types from `pg_catalog` to `pdb`
--
ALTER TYPE pg_catalog.fuzzy SET SCHEMA pdb;
ALTER TYPE pg_catalog.boost SET SCHEMA pdb;
ALTER TYPE pg_catalog.slop SET SCHEMA pdb;

--
-- relocate the paradedb.score function to the `pdb` schema
--
ALTER FUNCTION paradedb.score(anyelement) SET SCHEMA pdb;

--
-- relocate the paradedb.snippet* functions to the `pdb` schema
--
ALTER FUNCTION paradedb.snippet(anyelement, text, text, int, int, int) SET SCHEMA pdb;
ALTER FUNCTION paradedb.snippet_positions(anyelement, int, int) SET SCHEMA pdb;

--
-- drop the paradedb.snippets function, which comes into existence again as `pdb.snippets` in
-- `0.19.3`.
--
DROP FUNCTION IF EXISTS paradedb.snippets;

--
-- this begins the schema changes introduced by the new tokenizers-as-types SQL UX work
--

DROP OPERATOR IF EXISTS pg_catalog.&&&(text, text);
DROP OPERATOR IF EXISTS pg_catalog.&&&(text, pdb.boost);
DROP OPERATOR IF EXISTS pg_catalog.&&&(text, pdb.fuzzy);
DROP OPERATOR IF EXISTS pg_catalog.|||(text, text);
DROP OPERATOR IF EXISTS pg_catalog.|||(text, pdb.boost);
DROP OPERATOR IF EXISTS pg_catalog.|||(text, pdb.fuzzy);
DROP OPERATOR IF EXISTS pg_catalog.###(text, text);
DROP OPERATOR IF EXISTS pg_catalog.###(text, pdb.boost);
DROP OPERATOR IF EXISTS pg_catalog.===(text, text);
DROP OPERATOR IF EXISTS pg_catalog.===(text, text[]);
DROP OPERATOR IF EXISTS pg_catalog.===(text, pdb.boost);
DROP OPERATOR IF EXISTS pg_catalog.===(text, pdb.fuzzy);
DROP OPERATOR IF EXISTS pg_catalog.###(text, pdb.slop);
DROP OPERATOR IF EXISTS pg_catalog.&&&(text, pdb.query);
DROP OPERATOR IF EXISTS pg_catalog.===(text, pdb.query);
DROP OPERATOR IF EXISTS pg_catalog.|||(text, pdb.query);
DROP OPERATOR IF EXISTS pg_catalog.###(text, pdb.query);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/typmod/mod.rs:519

CREATE TABLE paradedb._typmod_cache(id SERIAL NOT NULL PRIMARY KEY, typmod text[] NOT NULL UNIQUE);
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache', '');
SELECT pg_catalog.pg_extension_config_dump('paradedb._typmod_cache_id_seq', '');
GRANT ALL ON TABLE paradedb._typmod_cache TO PUBLIC;
GRANT ALL ON SEQUENCE paradedb._typmod_cache_id_seq TO PUBLIC;
CREATE OR REPLACE FUNCTION paradedb._save_typmod(typmod_in text[])
    RETURNS integer SECURITY DEFINER STRICT VOLATILE PARALLEL UNSAFE
    LANGUAGE plpgsql AS $$
DECLARE
    v_id integer;
BEGIN
    INSERT INTO paradedb._typmod_cache (typmod)
    VALUES (typmod_in)
    ON CONFLICT (typmod) DO NOTHING
    RETURNING id INTO v_id;

    IF v_id IS NOT NULL THEN
        RETURN v_id;
    END IF;

    -- someone else inserted it concurrently, go read it again
    SELECT id INTO v_id
    FROM paradedb._typmod_cache
    WHERE typmod = typmod_in;

    IF v_id IS NULL THEN
        RAISE EXCEPTION 'typmod "%" not found after upsert', typmod_in;
    END IF;

    RETURN v_id;
END;
$$;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:312
-- pg_search::api::tokenizers::definitions::literal_typmod_in
CREATE  FUNCTION "literal_typmod_in"(
    "typmod_parts" cstring[] /* pgrx::datum::array::Array<&core::ffi::c_str::CStr> */
) RETURNS INT /* i32 */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'literal_typmod_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/typmod/mod.rs:42
-- pg_search::api::tokenizers::typmod::generic_typmod_in
CREATE  FUNCTION "generic_typmod_in"(
    "typmod_parts" cstring[] /* pgrx::datum::array::Array<&core::ffi::c_str::CStr> */
) RETURNS INT /* i32 */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'generic_typmod_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/typmod/mod.rs:47
-- pg_search::api::tokenizers::typmod::generic_typmod_out
CREATE  FUNCTION "generic_typmod_out"(
    "typmod" INT /* i32 */
) RETURNS cstring /* alloc::ffi::c_str::CString */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'generic_typmod_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:44
-- pg_search::api::builder_fns::pdb::pdb::_334ad99c0d964375b253cb4fc6e0a400::match_conjunction
CREATE  FUNCTION "match_conjunction"(
    "field" FieldName, /* pg_search::api::FieldName */
    "tokens" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_conjunction_array_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:69
-- pg_search::api::builder_fns::pdb::pdb::_91fbf011b0654370a8f98d6c7a204e61::match_disjunction
CREATE  FUNCTION "match_disjunction"(
    "field" FieldName, /* pg_search::api::FieldName */
    "tokens" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_disjunction_array_bfn_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:87
-- pg_search::api::builder_fns::pdb::pdb::_e5a146cb685e4669ad0f1ca8b1027515::phrase_array
CREATE  FUNCTION "phrase_array"(
    "field" FieldName, /* pg_search::api::FieldName */
    "tokens" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_array_bfn_wrapper';
DROP FUNCTION IF EXISTS search_with_match_conjunction(_field text, terms_to_tokenize text);
CREATE OR REPLACE FUNCTION search_with_match_conjunction(_field anyelement, terms_to_tokenize text) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_conjunction_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/andandand.rs:30
-- pg_search::api::operator::andandand::search_with_match_conjunction
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/andandand.rs:38
-- pg_search::api::operator::andandand::search_with_match_conjunction_array
CREATE  FUNCTION "search_with_match_conjunction_array"(
    "_field" anyelement, /* pgrx::datum::anyelement::AnyElement */
    "exact_tokens" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_conjunction_array_wrapper';
-- pg_search/src/api/operator/andandand.rs:38
-- pg_search::api::operator::andandand::search_with_match_conjunction_array
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction_array",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    );
DROP FUNCTION IF EXISTS search_with_match_conjunction_boost(_field text, terms_to_tokenize pdb.boost);
CREATE OR REPLACE FUNCTION search_with_match_conjunction_boost(_field anyelement, terms_to_tokenize pdb.boost) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_conjunction_boost_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/andandand.rs:57
-- pg_search::api::operator::andandand::search_with_match_conjunction_boost
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction_boost",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.boost /* pg_search::api::operator::boost::BoostType */
    );
DROP FUNCTION IF EXISTS search_with_match_conjunction_fuzzy(_field text, terms_to_tokenize pdb.fuzzy);
CREATE OR REPLACE FUNCTION search_with_match_conjunction_fuzzy(_field anyelement, terms_to_tokenize pdb.fuzzy) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_conjunction_fuzzy_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/andandand.rs:65
-- pg_search::api::operator::andandand::search_with_match_conjunction_fuzzy
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction_fuzzy",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
    );
DROP FUNCTION IF EXISTS search_with_match_disjunction(_field text, terms_to_tokenize text);
CREATE OR REPLACE FUNCTION search_with_match_disjunction(_field anyelement, terms_to_tokenize text) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_disjunction_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/ororor.rs:30
-- pg_search::api::operator::ororor::search_with_match_disjunction
CREATE OPERATOR pg_catalog.||| (
    PROCEDURE="search_with_match_disjunction",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/ororor.rs:38
-- pg_search::api::operator::ororor::search_with_match_disjunction_array
CREATE  FUNCTION "search_with_match_disjunction_array"(
    "_field" anyelement, /* pgrx::datum::anyelement::AnyElement */
    "exact_tokens" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_match_disjunction_array_wrapper';
-- pg_search/src/api/operator/ororor.rs:38
-- pg_search::api::operator::ororor::search_with_match_disjunction_array
CREATE OPERATOR pg_catalog.||| (
    PROCEDURE="search_with_match_disjunction_array",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    );
DROP FUNCTION IF EXISTS search_with_match_disjunction_boost(_field text, terms_to_tokenize pdb.boost);
CREATE OR REPLACE FUNCTION search_with_match_disjunction_boost(_field anyelement, terms_to_tokenize pdb.boost) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_disjunction_boost_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/ororor.rs:56
-- pg_search::api::operator::ororor::search_with_match_disjunction_boost
CREATE OPERATOR pg_catalog.||| (
    PROCEDURE="search_with_match_disjunction_boost",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.boost /* pg_search::api::operator::boost::BoostType */
    );
DROP FUNCTION IF EXISTS search_with_match_disjunction_fuzzy(_field text, terms_to_tokenize pdb.fuzzy);
CREATE OR REPLACE FUNCTION search_with_match_disjunction_fuzzy(_field anyelement, terms_to_tokenize pdb.fuzzy) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_disjunction_fuzzy_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/ororor.rs:63
-- pg_search::api::operator::ororor::search_with_match_disjunction_fuzzy
CREATE OPERATOR pg_catalog.||| (
    PROCEDURE="search_with_match_disjunction_fuzzy",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
    );
DROP FUNCTION IF EXISTS search_with_phrase(_field text, terms_to_tokenize text);
CREATE OR REPLACE FUNCTION search_with_phrase(_field anyelement, terms_to_tokenize text) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_phrase_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/hashhashhash.rs:30
-- pg_search::api::operator::hashhashhash::search_with_phrase
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=TEXT /* &str */
    );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/hashhashhash.rs:38
-- pg_search::api::operator::hashhashhash::search_with_phrase_array
CREATE  FUNCTION "search_with_phrase_array"(
    "_field" anyelement, /* pgrx::datum::anyelement::AnyElement */
    "tokens" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS bool /* bool */
    IMMUTABLE STRICT PARALLEL SAFE COST 1000000000
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_with_phrase_array_wrapper';
-- pg_search/src/api/operator/hashhashhash.rs:38
-- pg_search::api::operator::hashhashhash::search_with_phrase_array
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase_array",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    );
DROP FUNCTION IF EXISTS search_with_phrase_boost(_field text, terms_to_tokenize pdb.boost);
CREATE OR REPLACE FUNCTION search_with_phrase_boost(_field anyelement, terms_to_tokenize pdb.boost) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_phrase_boost_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/hashhashhash.rs:52
-- pg_search::api::operator::hashhashhash::search_with_phrase_boost
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase_boost",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.boost /* pg_search::api::operator::boost::BoostType */
    );
DROP FUNCTION IF EXISTS search_with_term(_field text, term text);
CREATE OR REPLACE FUNCTION search_with_term(_field anyelement, term text) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_term_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/eqeqeq.rs:30
-- pg_search::api::operator::eqeqeq::search_with_term
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=TEXT /* &str */
    );
DROP FUNCTION IF EXISTS search_with_term_array(_field text, terms text[]);
CREATE OR REPLACE FUNCTION search_with_term_array(_field anyelement, terms text[]) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_term_array_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/eqeqeq.rs:36
-- pg_search::api::operator::eqeqeq::search_with_term_array
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term_array",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    );
DROP FUNCTION IF EXISTS search_with_term_boost(_field text, term pdb.boost);
CREATE OR REPLACE FUNCTION search_with_term_boost(_field anyelement, term pdb.boost) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_term_boost_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/eqeqeq.rs:48
-- pg_search::api::operator::eqeqeq::search_with_term_boost
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term_boost",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.boost /* pg_search::api::operator::boost::BoostType */
    );
DROP FUNCTION IF EXISTS search_with_term_fuzzy(_field text, term pdb.fuzzy);
CREATE OR REPLACE FUNCTION search_with_term_fuzzy(_field anyelement, term pdb.fuzzy) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_term_fuzzy_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/eqeqeq.rs:54
-- pg_search::api::operator::eqeqeq::search_with_term_fuzzy
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term_fuzzy",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
    );
DROP FUNCTION IF EXISTS search_with_phrase_slop(_field text, terms_to_tokenize pdb.slop);
CREATE OR REPLACE FUNCTION search_with_phrase_slop(_field anyelement, terms_to_tokenize pdb.slop) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_phrase_slop_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/hashhashhash.rs:60
-- pg_search::api::operator::hashhashhash::search_with_phrase_slop
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase_slop",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.slop /* pg_search::api::operator::slop::SlopType */
    );
DROP FUNCTION IF EXISTS search_with_term_pdb_query(_field text, term pdb.query);
CREATE OR REPLACE FUNCTION search_with_term_pdb_query(_field anyelement, term pdb.query) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_term_pdb_query_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/eqeqeq.rs:42
-- pg_search::api::operator::eqeqeq::search_with_term_pdb_query
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term_pdb_query",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    );
DROP FUNCTION IF EXISTS search_with_match_disjunction_pdb_query(_field text, terms_to_tokenize pdb.query);
CREATE OR REPLACE FUNCTION search_with_match_disjunction_pdb_query(_field anyelement, terms_to_tokenize pdb.query) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_disjunction_pdb_query_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/ororor.rs:46
-- pg_search::api::operator::ororor::search_with_match_disjunction_pdb_query
CREATE OPERATOR pg_catalog.||| (
    PROCEDURE="search_with_match_disjunction_pdb_query",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    );
DROP FUNCTION IF EXISTS search_with_phrase_pdb_query(_field text, terms_to_tokenize pdb.query);
CREATE OR REPLACE FUNCTION search_with_phrase_pdb_query(_field anyelement, terms_to_tokenize pdb.query) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_phrase_pdb_query_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/hashhashhash.rs:44
-- pg_search::api::operator::hashhashhash::search_with_phrase_pdb_query
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase_pdb_query",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    );
DROP FUNCTION IF EXISTS search_with_match_conjunction_pdb_query(_field text, terms_to_tokenize pdb.query);
CREATE OR REPLACE FUNCTION search_with_match_conjunction_pdb_query(_field anyelement, terms_to_tokenize pdb.query) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_conjunction_pdb_query_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/andandand.rs:46
-- pg_search::api::operator::andandand::search_with_match_conjunction_pdb_query
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction_pdb_query",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:87
-- pg_search::api::builder_fns::pdb::pdb::phrase_array
CREATE  FUNCTION pdb."phrase_array"(
    "tokens" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'phrase_array_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:69
-- pg_search::api::builder_fns::pdb::pdb::match_disjunction
CREATE  FUNCTION pdb."match_disjunction"(
    "tokens" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_disjunction_array_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:44
-- pg_search::api::builder_fns::pdb::pdb::match_conjunction
CREATE  FUNCTION pdb."match_conjunction"(
    "tokens" TEXT[] /* alloc::vec::Vec<alloc::string::String> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'match_conjunction_array_wrapper';
/* pg_search::api::tokenizers::definitions::pdb */
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:214
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::Exact)


CREATE TYPE pdb.literal;
CREATE OR REPLACE FUNCTION pdb.literal_in(cstring) RETURNS pdb.literal AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.literal_out(pdb.literal) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.literal_send(pdb.literal) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.literal_recv(internal) RETURNS pdb.literal AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.literal (
                          INPUT = pdb.literal_in,
                          OUTPUT = pdb.literal_out,
                          SEND = pdb.literal_send,
                          RECEIVE = pdb.literal_recv,
                          COLLATABLE = true,
                          CATEGORY = 't', -- 't' is for tokenizer
                          PREFERRED = false,
                          LIKE = text
                      );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:329
-- requires:
--   literal_typmod_in
--   literal_definition


ALTER TYPE pdb.literal SET (TYPMOD_IN = literal_typmod_in);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:281
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::Ngram)


CREATE TYPE pdb.ngram;
CREATE OR REPLACE FUNCTION pdb.ngram_in(cstring) RETURNS pdb.ngram AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.ngram_out(pdb.ngram) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.ngram_send(pdb.ngram) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.ngram_recv(internal) RETURNS pdb.ngram AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.ngram (
                          INPUT = pdb.ngram_in,
                          OUTPUT = pdb.ngram_out,
                          SEND = pdb.ngram_send,
                          RECEIVE = pdb.ngram_recv,
                          COLLATABLE = true,
                          CATEGORY = 't', -- 't' is for tokenizer
                          PREFERRED = false,
                          LIKE = text
                      );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:203
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::Whitespace)


CREATE TYPE pdb.whitespace;
CREATE OR REPLACE FUNCTION pdb.whitespace_in(cstring) RETURNS pdb.whitespace AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.whitespace_out(pdb.whitespace) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.whitespace_send(pdb.whitespace) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.whitespace_recv(internal) RETURNS pdb.whitespace AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.whitespace (
                               INPUT = pdb.whitespace_in,
                               OUTPUT = pdb.whitespace_out,
                               SEND = pdb.whitespace_send,
                               RECEIVE = pdb.whitespace_recv,
                               COLLATABLE = true,
                               CATEGORY = 't', -- 't' is for tokenizer
                               PREFERRED = false,
                               LIKE = text
                           );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:181
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::Alias)


CREATE TYPE pdb.alias;
CREATE OR REPLACE FUNCTION pdb.alias_in(cstring) RETURNS pdb.alias AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.alias_out(pdb.alias) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.alias_send(pdb.alias) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.alias_recv(internal) RETURNS pdb.alias AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.alias (
                          INPUT = pdb.alias_in,
                          OUTPUT = pdb.alias_out,
                          SEND = pdb.alias_send,
                          RECEIVE = pdb.alias_recv,
                          COLLATABLE = true,
                          CATEGORY = 't', -- 't' is for tokenizer
                          PREFERRED = false,
                          LIKE = text
                      );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:192
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::Simple)


CREATE TYPE pdb.simple;
CREATE OR REPLACE FUNCTION pdb.simple_in(cstring) RETURNS pdb.simple AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.simple_out(pdb.simple) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.simple_send(pdb.simple) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.simple_recv(internal) RETURNS pdb.simple AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.simple (
                           INPUT = pdb.simple_in,
                           OUTPUT = pdb.simple_out,
                           SEND = pdb.simple_send,
                           RECEIVE = pdb.simple_recv,
                           COLLATABLE = true,
                           CATEGORY = 't', -- 't' is for tokenizer
                           PREFERRED = true,
                           LIKE = text
                       );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:192
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   simple_definition

ALTER TYPE pdb.simple SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:258
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::SourceCode)


CREATE TYPE pdb.source_code;
CREATE OR REPLACE FUNCTION pdb.source_code_in(cstring) RETURNS pdb.source_code AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.source_code_out(pdb.source_code) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.source_code_send(pdb.source_code) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.source_code_recv(internal) RETURNS pdb.source_code AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.source_code (
                                INPUT = pdb.source_code_in,
                                OUTPUT = pdb.source_code_out,
                                SEND = pdb.source_code_send,
                                RECEIVE = pdb.source_code_recv,
                                COLLATABLE = true,
                                CATEGORY = 't', -- 't' is for tokenizer
                                PREFERRED = false,
                                LIKE = text
                            );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:236
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::Lindera)


CREATE TYPE pdb.lindera;
CREATE OR REPLACE FUNCTION pdb.lindera_in(cstring) RETURNS pdb.lindera AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.lindera_out(pdb.lindera) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.lindera_send(pdb.lindera) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.lindera_recv(internal) RETURNS pdb.lindera AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.lindera (
                            INPUT = pdb.lindera_in,
                            OUTPUT = pdb.lindera_out,
                            SEND = pdb.lindera_send,
                            RECEIVE = pdb.lindera_recv,
                            COLLATABLE = true,
                            CATEGORY = 't', -- 't' is for tokenizer
                            PREFERRED = false,
                            LIKE = text
                        );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::ChineseCompatible)


CREATE TYPE pdb.chinese_compatible;
CREATE OR REPLACE FUNCTION pdb.chinese_compatible_in(cstring) RETURNS pdb.chinese_compatible AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.chinese_compatible_out(pdb.chinese_compatible) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.chinese_compatible_send(pdb.chinese_compatible) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.chinese_compatible_recv(internal) RETURNS pdb.chinese_compatible AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.chinese_compatible (
                                       INPUT = pdb.chinese_compatible_in,
                                       OUTPUT = pdb.chinese_compatible_out,
                                       SEND = pdb.chinese_compatible_send,
                                       RECEIVE = pdb.chinese_compatible_recv,
                                       COLLATABLE = true,
                                       CATEGORY = 't', -- 't' is for tokenizer
                                       PREFERRED = false,
                                       LIKE = text
                                   );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:181
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   alias_definition

ALTER TYPE pdb.alias SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   chinese_compatible_definition

ALTER TYPE pdb.chinese_compatible SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:258
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   source_code_definition

ALTER TYPE pdb.source_code SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:203
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   whitespace_definition

ALTER TYPE pdb.whitespace SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:281
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   ngram_definition

ALTER TYPE pdb.ngram SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:236
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   lindera_definition

ALTER TYPE pdb.lindera SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:247
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::Jieba)


CREATE TYPE pdb.jieba;
CREATE OR REPLACE FUNCTION pdb.jieba_in(cstring) RETURNS pdb.jieba AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.jieba_out(pdb.jieba) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.jieba_send(pdb.jieba) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.jieba_recv(internal) RETURNS pdb.jieba AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.jieba (
                          INPUT = pdb.jieba_in,
                          OUTPUT = pdb.jieba_out,
                          SEND = pdb.jieba_send,
                          RECEIVE = pdb.jieba_recv,
                          COLLATABLE = true,
                          CATEGORY = 't', -- 't' is for tokenizer
                          PREFERRED = false,
                          LIKE = text
                      );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:247
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   jieba_definition

ALTER TYPE pdb.jieba SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::Regex)


CREATE TYPE pdb.regex_pattern;
CREATE OR REPLACE FUNCTION pdb.regex_in(cstring) RETURNS pdb.regex_pattern AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.regex_out(pdb.regex_pattern) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.regex_send(pdb.regex_pattern) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.regex_recv(internal) RETURNS pdb.regex_pattern AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.regex_pattern (
                          INPUT = pdb.regex_in,
                          OUTPUT = pdb.regex_out,
                          SEND = pdb.regex_send,
                          RECEIVE = pdb.regex_recv,
                          COLLATABLE = true,
                          CATEGORY = 't', -- 't' is for tokenizer
                          PREFERRED = false,
                          LIKE = text
                      );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   regex_definition

ALTER TYPE pdb.regex_pattern SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:214
-- pg_search::api::tokenizers::definitions::pdb::tokenize_literal
CREATE  FUNCTION pdb."tokenize_literal"(
    "s" pdb.literal /* pg_search::api::tokenizers::definitions::pdb::Exact */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_literal_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:214
-- requires:
--   literal_definition
--   tokenize_literal

CREATE CAST (pdb.literal AS TEXT[]) WITH FUNCTION pdb.tokenize_literal AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:258
-- pg_search::api::tokenizers::definitions::pdb::tokenize_source_code
CREATE  FUNCTION pdb."tokenize_source_code"(
    "s" pdb.source_code /* pg_search::api::tokenizers::definitions::pdb::SourceCode */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_source_code_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:258
-- requires:
--   source_code_definition
--   tokenize_source_code

CREATE CAST (pdb.source_code AS TEXT[]) WITH FUNCTION pdb.tokenize_source_code AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:192
-- pg_search::api::tokenizers::definitions::pdb::tokenize_simple
CREATE  FUNCTION pdb."tokenize_simple"(
    "s" pdb.simple /* pg_search::api::tokenizers::definitions::pdb::Simple */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_simple_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:192
-- requires:
--   simple_definition
--   tokenize_simple

CREATE CAST (pdb.simple AS TEXT[]) WITH FUNCTION pdb.tokenize_simple AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:192
-- pg_search::api::tokenizers::definitions::pdb::json_to_simple
-- requires:
--   tokenize_simple
CREATE  FUNCTION pdb."json_to_simple"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.simple /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Simple> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_simple_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- pg_search::api::tokenizers::definitions::pdb::tokenize_regex
CREATE  FUNCTION pdb."tokenize_regex"(
    "s" pdb.regex_pattern /* pg_search::api::tokenizers::definitions::pdb::Regex */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_regex_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- requires:
--   regex_definition
--   tokenize_regex

CREATE CAST (pdb.regex_pattern AS TEXT[]) WITH FUNCTION pdb.tokenize_regex AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- pg_search::api::tokenizers::definitions::pdb::json_to_regex
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."json_to_regex"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.regex_pattern /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Regex> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_regex_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:236
-- pg_search::api::tokenizers::definitions::pdb::tokenize_lindera
CREATE  FUNCTION pdb."tokenize_lindera"(
    "s" pdb.lindera /* pg_search::api::tokenizers::definitions::pdb::Lindera */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_lindera_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:236
-- requires:
--   lindera_definition
--   tokenize_lindera

CREATE CAST (pdb.lindera AS TEXT[]) WITH FUNCTION pdb.tokenize_lindera AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:236
-- pg_search::api::tokenizers::definitions::pdb::json_to_lindera
-- requires:
--   tokenize_lindera
CREATE  FUNCTION pdb."json_to_lindera"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.lindera /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Lindera> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_lindera_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:236
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_lindera
-- requires:
--   tokenize_lindera
CREATE  FUNCTION pdb."jsonb_to_lindera"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.lindera /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Lindera> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_lindera_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:236
-- requires:
--   lindera_definition
--   json_to_lindera
--   jsonb_to_lindera


CREATE CAST (json AS pdb.lindera) WITH FUNCTION pdb.json_to_lindera AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.lindera) WITH FUNCTION pdb.jsonb_to_lindera AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:214
-- pg_search::api::tokenizers::definitions::pdb::json_to_literal
-- requires:
--   tokenize_literal
CREATE  FUNCTION pdb."json_to_literal"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.literal /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Exact> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_literal_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:203
-- pg_search::api::tokenizers::definitions::pdb::tokenize_whitespace
CREATE  FUNCTION pdb."tokenize_whitespace"(
    "s" pdb.whitespace /* pg_search::api::tokenizers::definitions::pdb::Whitespace */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_whitespace_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:203
-- requires:
--   whitespace_definition
--   tokenize_whitespace

CREATE CAST (pdb.whitespace AS TEXT[]) WITH FUNCTION pdb.tokenize_whitespace AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:203
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_whitespace
-- requires:
--   tokenize_whitespace
CREATE  FUNCTION pdb."jsonb_to_whitespace"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.whitespace /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Whitespace> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_whitespace_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:192
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_simple
-- requires:
--   tokenize_simple
CREATE  FUNCTION pdb."jsonb_to_simple"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.simple /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Simple> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_simple_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:192
-- requires:
--   simple_definition
--   json_to_simple
--   jsonb_to_simple


CREATE CAST (json AS pdb.simple) WITH FUNCTION pdb.json_to_simple AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.simple) WITH FUNCTION pdb.jsonb_to_simple AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:214
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_literal
-- requires:
--   tokenize_literal
CREATE  FUNCTION pdb."jsonb_to_literal"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.literal /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Exact> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_literal_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:214
-- requires:
--   literal_definition
--   json_to_literal
--   jsonb_to_literal


CREATE CAST (json AS pdb.literal) WITH FUNCTION pdb.json_to_literal AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.literal) WITH FUNCTION pdb.jsonb_to_literal AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_regex
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."jsonb_to_regex"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.regex_pattern /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Regex> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_regex_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- requires:
--   regex_definition
--   json_to_regex
--   jsonb_to_regex


CREATE CAST (json AS pdb.regex_pattern) WITH FUNCTION pdb.json_to_regex AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.regex_pattern) WITH FUNCTION pdb.jsonb_to_regex AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:181
-- pg_search::api::tokenizers::definitions::pdb::tokenize_alias
CREATE  FUNCTION pdb."tokenize_alias"(
    "s" pdb.alias /* pg_search::api::tokenizers::definitions::pdb::Alias */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_alias_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:181
-- requires:
--   alias_definition
--   tokenize_alias

CREATE CAST (pdb.alias AS TEXT[]) WITH FUNCTION pdb.tokenize_alias AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- pg_search::api::tokenizers::definitions::pdb::tokenize_chinese_compatible
CREATE  FUNCTION pdb."tokenize_chinese_compatible"(
    "s" pdb.chinese_compatible /* pg_search::api::tokenizers::definitions::pdb::ChineseCompatible */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_chinese_compatible_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- requires:
--   chinese_compatible_definition
--   tokenize_chinese_compatible

CREATE CAST (pdb.chinese_compatible AS TEXT[]) WITH FUNCTION pdb.tokenize_chinese_compatible AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_chinese_compatible
-- requires:
--   tokenize_chinese_compatible
CREATE  FUNCTION pdb."jsonb_to_chinese_compatible"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.chinese_compatible /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::ChineseCompatible> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_chinese_compatible_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- pg_search::api::tokenizers::definitions::pdb::json_to_chinese_compatible
-- requires:
--   tokenize_chinese_compatible
CREATE  FUNCTION pdb."json_to_chinese_compatible"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.chinese_compatible /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::ChineseCompatible> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_chinese_compatible_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- requires:
--   chinese_compatible_definition
--   json_to_chinese_compatible
--   jsonb_to_chinese_compatible


CREATE CAST (json AS pdb.chinese_compatible) WITH FUNCTION pdb.json_to_chinese_compatible AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.chinese_compatible) WITH FUNCTION pdb.jsonb_to_chinese_compatible AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:258
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_source_code
-- requires:
--   tokenize_source_code
CREATE  FUNCTION pdb."jsonb_to_source_code"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.source_code /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::SourceCode> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_source_code_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:258
-- pg_search::api::tokenizers::definitions::pdb::json_to_source_code
-- requires:
--   tokenize_source_code
CREATE  FUNCTION pdb."json_to_source_code"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.source_code /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::SourceCode> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_source_code_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:258
-- requires:
--   source_code_definition
--   json_to_source_code
--   jsonb_to_source_code


CREATE CAST (json AS pdb.source_code) WITH FUNCTION pdb.json_to_source_code AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.source_code) WITH FUNCTION pdb.jsonb_to_source_code AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:203
-- pg_search::api::tokenizers::definitions::pdb::json_to_whitespace
-- requires:
--   tokenize_whitespace
CREATE  FUNCTION pdb."json_to_whitespace"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.whitespace /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Whitespace> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_whitespace_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:203
-- requires:
--   whitespace_definition
--   json_to_whitespace
--   jsonb_to_whitespace


CREATE CAST (json AS pdb.whitespace) WITH FUNCTION pdb.json_to_whitespace AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.whitespace) WITH FUNCTION pdb.jsonb_to_whitespace AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:181
-- pg_search::api::tokenizers::definitions::pdb::json_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."json_to_alias"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_alias_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:181
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_alias
-- requires:
--   tokenize_alias
CREATE  FUNCTION pdb."jsonb_to_alias"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.alias /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Alias> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_alias_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:181
-- requires:
--   alias_definition
--   json_to_alias
--   jsonb_to_alias


CREATE CAST (json AS pdb.alias) WITH FUNCTION pdb.json_to_alias AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.alias) WITH FUNCTION pdb.jsonb_to_alias AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:247
-- pg_search::api::tokenizers::definitions::pdb::tokenize_jieba
CREATE  FUNCTION pdb."tokenize_jieba"(
    "s" pdb.jieba /* pg_search::api::tokenizers::definitions::pdb::Jieba */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_jieba_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:247
-- requires:
--   jieba_definition
--   tokenize_jieba

CREATE CAST (pdb.jieba AS TEXT[]) WITH FUNCTION pdb.tokenize_jieba AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:247
-- pg_search::api::tokenizers::definitions::pdb::json_to_jieba
-- requires:
--   tokenize_jieba
CREATE  FUNCTION pdb."json_to_jieba"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.jieba /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Jieba> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_jieba_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:247
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_jieba
-- requires:
--   tokenize_jieba
CREATE  FUNCTION pdb."jsonb_to_jieba"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.jieba /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Jieba> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_jieba_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:247
-- requires:
--   jieba_definition
--   json_to_jieba
--   jsonb_to_jieba


CREATE CAST (json AS pdb.jieba) WITH FUNCTION pdb.json_to_jieba AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.jieba) WITH FUNCTION pdb.jsonb_to_jieba AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:281
-- pg_search::api::tokenizers::definitions::pdb::tokenize_ngram
CREATE  FUNCTION pdb."tokenize_ngram"(
    "s" pdb.ngram /* pg_search::api::tokenizers::definitions::pdb::Ngram */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_ngram_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:281
-- requires:
--   ngram_definition
--   tokenize_ngram

CREATE CAST (pdb.ngram AS TEXT[]) WITH FUNCTION pdb.tokenize_ngram AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:281
-- pg_search::api::tokenizers::definitions::pdb::json_to_ngram
-- requires:
--   tokenize_ngram
CREATE  FUNCTION pdb."json_to_ngram"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.ngram /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Ngram> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_ngram_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:281
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_ngram
-- requires:
--   tokenize_ngram
CREATE  FUNCTION pdb."jsonb_to_ngram"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.ngram /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Ngram> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_ngram_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:281
-- requires:
--   ngram_definition
--   json_to_ngram
--   jsonb_to_ngram


CREATE CAST (json AS pdb.ngram) WITH FUNCTION pdb.json_to_ngram AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.ngram) WITH FUNCTION pdb.jsonb_to_ngram AS ASSIGNMENT;

--
-- text[] casting to boost/fuzzy/slop on the rhs of some operators
--

/* <begin connected objects> */
-- pg_search/src/api/operator/boost.rs:213
-- pg_search::api::operator::boost::text_array_to_boost
CREATE  FUNCTION "text_array_to_boost"(
    "array" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "typmod" INT, /* i32 */
    "_is_explicit" bool /* bool */
) RETURNS pdb.boost /* pg_search::api::operator::boost::BoostType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_boost_wrapper';
/* </end connected objects> */


-- pg_search/src/api/operator/fuzzy.rs:215
-- pg_search::api::operator::fuzzy::text_array_to_fuzzy
CREATE  FUNCTION "text_array_to_fuzzy"(
    "array" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "typmod" INT, /* i32 */
    "_is_explicit" bool /* bool */
) RETURNS pdb.fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_fuzzy_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:191
-- pg_search::api::operator::slop::text_array_to_slop
CREATE  FUNCTION "text_array_to_slop"(
    "array" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "typmod" INT, /* i32 */
    "_is_explicit" bool /* bool */
) RETURNS pdb.slop /* pg_search::api::operator::slop::SlopType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_slop_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/fuzzy.rs:255
-- requires:
--   query_to_fuzzy
--   fuzzy_to_boost
--   fuzzy_to_fuzzy
--   text_array_to_fuzzy
--   FuzzyType_final
CREATE CAST (text[] AS pdb.fuzzy) WITH FUNCTION text_array_to_fuzzy(text[], integer, boolean) AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/slop.rs:231
-- requires:
--   query_to_slop
--   slop_to_boost
--   slop_to_slop
--   text_array_to_slop
--   SlopType_final
CREATE CAST (text[] AS pdb.slop) WITH FUNCTION text_array_to_slop(text[], integer, boolean) AS ASSIGNMENT;

/* <begin connected objects> */
-- pg_search/src/api/operator/boost.rs:286
-- requires:
--   query_to_boost
--   prox_to_boost
--   boost_to_boost
--   text_array_to_boost
--   BoostType_final
CREATE CAST (text[] AS pdb.boost) WITH FUNCTION text_array_to_boost(text[], integer, boolean) AS ASSIGNMENT;
/* pg_search::api::tokenizers::definitions::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::LiteralNormalized)
            CREATE TYPE pdb.literal_normalized;
CREATE OR REPLACE FUNCTION pdb.literal_normalized_in(cstring) RETURNS pdb.literal_normalized AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.literal_normalized_out(pdb.literal_normalized) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.literal_normalized_send(pdb.literal_normalized) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.literal_normalized_recv(internal) RETURNS pdb.literal_normalized AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.literal_normalized (
                INPUT = pdb.literal_normalized_in,
                OUTPUT = pdb.literal_normalized_out,
                SEND = pdb.literal_normalized_send,
                RECEIVE = pdb.literal_normalized_recv,
                COLLATABLE = true,
                CATEGORY = 't', -- 't' is for tokenizer
                PREFERRED = false,
                LIKE = text
            );
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   literal_normalized_definition
ALTER TYPE pdb.literal_normalized SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- pg_search::api::tokenizers::definitions::pdb::tokenize_literal_normalized
CREATE  FUNCTION pdb."tokenize_literal_normalized"(
	"s" pdb.literal_normalized /* pg_search::api::tokenizers::definitions::pdb::LiteralNormalized */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_literal_normalized_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- requires:
--   literal_normalized_definition
--   tokenize_literal_normalized
CREATE CAST (pdb.literal_normalized AS TEXT[]) WITH FUNCTION pdb.tokenize_literal_normalized AS IMPLICIT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_literal_normalized
-- requires:
--   tokenize_literal_normalized
CREATE  FUNCTION pdb."jsonb_to_literal_normalized"(
	"jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.literal_normalized /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::LiteralNormalized> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_literal_normalized_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- pg_search::api::tokenizers::definitions::pdb::json_to_literal_normalized
-- requires:
--   tokenize_literal_normalized
CREATE  FUNCTION pdb."json_to_literal_normalized"(
	"json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.literal_normalized /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::LiteralNormalized> */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_literal_normalized_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:225
-- requires:
--   literal_normalized_definition
--   json_to_literal_normalized
--   jsonb_to_literal_normalized
        CREATE CAST (json AS pdb.literal_normalized) WITH FUNCTION pdb.json_to_literal_normalized AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.literal_normalized) WITH FUNCTION pdb.jsonb_to_literal_normalized AS ASSIGNMENT;

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:322
-- creates:
--   Type(pg_search::api::tokenizers::definitions::pdb::UnicodeWords)
CREATE TYPE pdb.unicode_words;
CREATE OR REPLACE FUNCTION pdb.unicode_words_in(cstring) RETURNS pdb.unicode_words AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.unicode_words_out(pdb.unicode_words) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.unicode_words_send(pdb.unicode_words) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.unicode_words_recv(internal) RETURNS pdb.unicode_words AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.unicode_words (
                                  INPUT = pdb.unicode_words_in,
                                  OUTPUT = pdb.unicode_words_out,
                                  SEND = pdb.unicode_words_send,
                                  RECEIVE = pdb.unicode_words_recv,
                                  COLLATABLE = true,
                                  CATEGORY = 't', -- 't' is for tokenizer
                                  PREFERRED = false,
                                  LIKE = text
                              );
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:322
-- requires:
--   generic_typmod_in
--   generic_typmod_out
--   unicode_words_definition
ALTER TYPE pdb.unicode_words SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:322
-- pg_search::api::tokenizers::definitions::pdb::tokenize_unicode_words
CREATE  FUNCTION pdb."tokenize_unicode_words"(
    "s" pdb.unicode_words /* pg_search::api::tokenizers::definitions::pdb::UnicodeWords */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_unicode_words_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:322
-- requires:
--   unicode_words_definition
--   tokenize_unicode_words
CREATE CAST (pdb.unicode_words AS TEXT[]) WITH FUNCTION pdb.tokenize_unicode_words AS IMPLICIT;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:322
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_unicode_words
-- requires:
--   tokenize_unicode_words
CREATE  FUNCTION pdb."jsonb_to_unicode_words"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.unicode_words /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::UnicodeWords> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_unicode_words_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:322
-- pg_search::api::tokenizers::definitions::pdb::json_to_unicode_words
-- requires:
--   tokenize_unicode_words
CREATE  FUNCTION pdb."json_to_unicode_words"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.unicode_words /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::UnicodeWords> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_unicode_words_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:322
-- requires:
--   unicode_words_definition
--   json_to_unicode_words
--   jsonb_to_unicode_words
CREATE CAST (json AS pdb.unicode_words) WITH FUNCTION pdb.json_to_unicode_words AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.unicode_words) WITH FUNCTION pdb.jsonb_to_unicode_words AS ASSIGNMENT;


--
-- pdb.all() and pdb.empty()
--

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:36
-- pg_search::api::builder_fns::pdb::pdb::empty
CREATE  FUNCTION pdb."empty"() RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'pdb_empty_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:31
-- pg_search::api::builder_fns::pdb::pdb::all
CREATE  FUNCTION pdb."all"() RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'pdb_all_wrapper';
/* </end connected objects> */


--
-- the ::pdb.const type, akin to ::pdb.boost
--

CREATE TYPE pdb.const;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:140
-- pg_search::api::operator::const_score::typedef::const_in
CREATE  FUNCTION "const_in"(
    "input" cstring, /* &core::ffi::c_str::CStr */
    "_typoid" oid, /* pgrx_pg_sys::submodules::oids::Oid */
    "typmod" INT /* i32 */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'const_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:152
-- pg_search::api::operator::const_score::typedef::const_out
CREATE  FUNCTION "const_out"(
    "input" pdb.const /* pg_search::api::operator::const_score::ConstType */
) RETURNS cstring /* alloc::ffi::c_str::CString */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'const_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:272
-- pg_search::api::operator::const_score::const_to_const
CREATE  FUNCTION "const_to_const"(
    "input" pdb.const, /* pg_search::api::operator::const_score::ConstType */
    "typmod" INT, /* i32 */
    "_is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'const_to_const_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:163
-- pg_search::api::operator::const_score::typedef::const_typmod_in
CREATE  FUNCTION "const_typmod_in"(
    "typmod_parts" cstring[] /* pgrx::datum::array::Array<&core::ffi::c_str::CStr> */
) RETURNS INT /* i32 */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'const_typmod_in_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:177
-- pg_search::api::operator::const_score::typedef::const_typmod_out
CREATE  FUNCTION "const_typmod_out"(
    "typmod" INT /* i32 */
) RETURNS cstring /* alloc::ffi::c_str::CString */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'const_typmod_out_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:183
-- requires:
--   ConstType_shell
--   const_in
--   const_out
--   const_typmod_in
--   const_typmod_out


CREATE TYPE pdb.const (
                          INPUT = const_in,
                          OUTPUT = const_out,
                          INTERNALLENGTH = VARIABLE,
                          LIKE = text,
                          TYPMOD_IN = const_typmod_in,
                          TYPMOD_OUT = const_typmod_out
                      );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:214
-- pg_search::api::operator::const_score::text_array_to_const
CREATE  FUNCTION "text_array_to_const"(
    "array" TEXT[], /* alloc::vec::Vec<alloc::string::String> */
    "typmod" INT, /* i32 */
    "_is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'text_array_to_const_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:205
-- pg_search::api::operator::const_score::query_to_const
CREATE  FUNCTION "query_to_const"(
    "input" pdb.Query, /* pg_search::query::pdb_query::pdb::Query */
    "typmod" INT, /* i32 */
    "_is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'query_to_const_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:253
-- pg_search::api::operator::const_score::const_to_query
CREATE  FUNCTION "const_to_query"(
    "input" pdb.const /* pg_search::api::operator::const_score::ConstType */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'const_to_query_wrapper';
-- pg_search/src/api/operator/const_score.rs:253
-- pg_search::api::operator::const_score::const_to_query
CREATE CAST (
    pdb.const /* pg_search::api::operator::const_score::ConstType */
    AS
    pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    )
    WITH FUNCTION const_to_query AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:228
-- pg_search::api::operator::const_score::prox_to_const
CREATE  FUNCTION "prox_to_const"(
    "input" pdb.ProximityClause, /* pg_search::query::proximity::pdb::ProximityClause */
    "typmod" INT, /* i32 */
    "_is_explicit" bool /* bool */
) RETURNS pdb.const /* pg_search::api::operator::const_score::ConstType */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'prox_to_const_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/operator/const_score.rs:287
-- requires:
--   query_to_const
--   prox_to_const
--   const_to_const
--   text_array_to_const
--   ConstType_final


CREATE CAST (text[] AS pdb.const) WITH FUNCTION text_array_to_const(text[], integer, boolean) AS ASSIGNMENT;
CREATE CAST (pdb.query AS pdb.const) WITH FUNCTION query_to_const(pdb.query, integer, boolean) AS ASSIGNMENT;
CREATE CAST (pdb.proximityclause AS pdb.const) WITH FUNCTION prox_to_const(pdb.proximityclause, integer, boolean) AS ASSIGNMENT;
CREATE CAST (pdb.const AS pdb.const) WITH FUNCTION const_to_const(pdb.const, integer, boolean) AS IMPLICIT;

DROP FUNCTION IF EXISTS pdb.more_like_this(document_fields text, min_doc_frequency pg_catalog.int4, max_doc_frequency pg_catalog.int4, min_term_frequency pg_catalog.int4, max_query_terms pg_catalog.int4, min_word_length pg_catalog.int4, max_word_length pg_catalog.int4, boost_factor pg_catalog.float4, stop_words text[]);
CREATE OR REPLACE FUNCTION pdb.more_like_this(document text, min_doc_frequency pg_catalog.int4 DEFAULT NULL, max_doc_frequency pg_catalog.int4 DEFAULT NULL, min_term_frequency pg_catalog.int4 DEFAULT NULL, max_query_terms pg_catalog.int4 DEFAULT NULL, min_word_length pg_catalog.int4 DEFAULT NULL, max_word_length pg_catalog.int4 DEFAULT NULL, boost_factor pg_catalog.float4 DEFAULT NULL, stopwords text[] DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'more_like_this_fields_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;
DROP FUNCTION IF EXISTS pdb.more_like_this(document_id anyelement, min_doc_frequency pg_catalog.int4, max_doc_frequency pg_catalog.int4, min_term_frequency pg_catalog.int4, max_query_terms pg_catalog.int4, min_word_length pg_catalog.int4, max_word_length pg_catalog.int4, boost_factor pg_catalog.float4, stop_words text[]);
CREATE OR REPLACE FUNCTION pdb.more_like_this(key_value anyelement, fields text[] DEFAULT NULL, min_doc_frequency pg_catalog.int4 DEFAULT NULL, max_doc_frequency pg_catalog.int4 DEFAULT NULL, min_term_frequency pg_catalog.int4 DEFAULT NULL, max_query_terms pg_catalog.int4 DEFAULT NULL, min_word_length pg_catalog.int4 DEFAULT NULL, max_word_length pg_catalog.int4 DEFAULT NULL, boost_factor pg_catalog.float4 DEFAULT NULL, stopwords text[] DEFAULT NULL) RETURNS searchqueryinput AS 'MODULE_PATHNAME', 'more_like_this_id_wrapper' IMMUTABLE LANGUAGE c PARALLEL SAFE;

DROP FUNCTION IF EXISTS paradedb.index_info(index regclass, show_invisible bool) CASCADE;
CREATE  FUNCTION paradedb.index_info(
	"index" regclass, /* pgrx::rel::PgRelation */
	"show_invisible" bool DEFAULT false /* bool */
) RETURNS TABLE (
	"index_name" TEXT,  /* alloc::string::String */
	"visible" bool,  /* bool */
	"recyclable" bool,  /* bool */
	"xmax" xid,  /* pgrx_pg_sys::submodules::transaction_id::TransactionId */
	"segno" TEXT,  /* alloc::string::String */
	"mutable" bool,  /* bool */
	"byte_size" NUMERIC,  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
	"num_docs" NUMERIC,  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
	"num_deleted" NUMERIC,  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
	"termdict_bytes" NUMERIC,  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
	"postings_bytes" NUMERIC,  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
	"positions_bytes" NUMERIC,  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
	"fast_fields_bytes" NUMERIC,  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
	"fieldnorms_bytes" NUMERIC,  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
	"store_bytes" NUMERIC,  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
	"deletes_bytes" NUMERIC  /* core::option::Option<pgrx::datum::numeric::AnyNumeric> */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'index_info_wrapper';

create view paradedb.index_layer_info as
select relname::text,
       layer_size,
       low,
       high,
       byte_size,
       case when segments = ARRAY [NULL] then 0 else count end       as count,
       case when segments = ARRAY [NULL] then NULL else segments end as segments
from (select relname,
             coalesce(pg_size_pretty(case when low = 0 then null else low end), '') || '..' ||
             coalesce(pg_size_pretty(case when high = 9223372036854775807 then null else high end), '') as layer_size,
             count(*),
             coalesce(sum(byte_size), 0)                                                                as byte_size,
             min(low)                                                                                   as low,
             max(high)                                                                                  as high,
             array_agg(segno)                                                                           as segments
      from (with indexes as (select oid::regclass as relname
                             from pg_class
                             where relam = (select oid from pg_am where amname = 'bm25')),
                 segments as (select relname, index_info.*
                              from indexes
                                       inner join paradedb.index_info(indexes.relname, true) on true),
                 layer_sizes as (select relname, coalesce(lead(unnest) over (), 0) low, unnest as high
                                 from indexes
                                          inner join lateral (select unnest(0 || paradedb.layer_sizes(indexes.relname) || 9223372036854775807)
                                                              order by 1 desc) x on true)
            select layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size
            from layer_sizes
                     left join segments on layer_sizes.relname = segments.relname and
                                           (byte_size * 1.33)::bigint between low and high) x
      where low < high
      group by relname, low, high
      order by relname, low desc) x;

GRANT SELECT ON paradedb.index_layer_info TO PUBLIC;

ALTER FUNCTION paradedb.search_with_phrase SUPPORT paradedb.search_with_phrase_support;
ALTER FUNCTION paradedb.search_with_phrase_array SUPPORT paradedb.search_with_phrase_support;
ALTER FUNCTION paradedb.search_with_phrase_pdb_query SUPPORT paradedb.search_with_phrase_support;
ALTER FUNCTION paradedb.search_with_phrase_boost SUPPORT paradedb.search_with_phrase_support;
ALTER FUNCTION paradedb.search_with_phrase_slop SUPPORT paradedb.search_with_phrase_support;

ALTER FUNCTION paradedb.search_with_match_conjunction SUPPORT paradedb.search_with_match_conjunction_support;
ALTER FUNCTION paradedb.search_with_match_conjunction_array SUPPORT paradedb.search_with_match_conjunction_support;
ALTER FUNCTION paradedb.search_with_match_conjunction_pdb_query SUPPORT paradedb.search_with_match_conjunction_support;
ALTER FUNCTION paradedb.search_with_match_conjunction_boost SUPPORT paradedb.search_with_match_conjunction_support;
ALTER FUNCTION paradedb.search_with_match_conjunction_fuzzy SUPPORT paradedb.search_with_match_conjunction_support;

ALTER FUNCTION paradedb.search_with_match_disjunction SUPPORT paradedb.search_with_match_disjunction_support;
ALTER FUNCTION paradedb.search_with_match_disjunction_array SUPPORT paradedb.search_with_match_disjunction_support;
ALTER FUNCTION paradedb.search_with_match_disjunction_pdb_query SUPPORT paradedb.search_with_match_disjunction_support;
ALTER FUNCTION paradedb.search_with_match_disjunction_boost SUPPORT paradedb.search_with_match_disjunction_support;
ALTER FUNCTION paradedb.search_with_match_disjunction_fuzzy SUPPORT paradedb.search_with_match_disjunction_support;

ALTER FUNCTION paradedb.search_with_term SUPPORT paradedb.search_with_term_support;
ALTER FUNCTION paradedb.search_with_term_array SUPPORT paradedb.search_with_term_support;
ALTER FUNCTION paradedb.search_with_term_pdb_query SUPPORT paradedb.search_with_term_support;
ALTER FUNCTION paradedb.search_with_term_boost SUPPORT paradedb.search_with_term_support;
ALTER FUNCTION paradedb.search_with_term_fuzzy SUPPORT paradedb.search_with_term_support;

ALTER FUNCTION paradedb.search_with_parse SUPPORT paradedb.atatat_support;
ALTER FUNCTION paradedb.search_with_fieled_query_input SUPPORT paradedb.atatat_support;
ALTER FUNCTION paradedb.search_with_proximity_clause SUPPORT paradedb.atatat_support;

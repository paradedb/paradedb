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
-- this begins the schema changes introduced by the new tokenizers-as-types SQL UX work
--

DROP OPERATOR IF EXISTS pg_catalog."&&&"(text, text);
DROP OPERATOR IF EXISTS pg_catalog."&&&"(text, boost);
DROP OPERATOR IF EXISTS pg_catalog."&&&"(text, fuzzy);
DROP OPERATOR IF EXISTS pg_catalog."|||"(text, text);
DROP OPERATOR IF EXISTS pg_catalog."|||"(text, boost);
DROP OPERATOR IF EXISTS pg_catalog."|||"(text, fuzzy);
DROP OPERATOR IF EXISTS pg_catalog."###"(text, text);
DROP OPERATOR IF EXISTS pg_catalog."###"(text, boost);
DROP OPERATOR IF EXISTS pg_catalog."==="(text, text);
DROP OPERATOR IF EXISTS pg_catalog."==="(text, text[]);
DROP OPERATOR IF EXISTS pg_catalog."==="(text, boost);
DROP OPERATOR IF EXISTS pg_catalog."==="(text, fuzzy);
DROP OPERATOR IF EXISTS pg_catalog."###"(text, slop);
DROP OPERATOR IF EXISTS pg_catalog."&&&"(text, pdb.query);
DROP OPERATOR IF EXISTS pg_catalog."==="(text, pdb.query);
DROP OPERATOR IF EXISTS pg_catalog."|||"(text, pdb.query);
DROP OPERATOR IF EXISTS pg_catalog."###"(text, pdb.query);
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
-- pg_search::api::tokenizers::definitions::exact_typmod_in
CREATE  FUNCTION "exact_typmod_in"(
    "typmod_parts" cstring[] /* pgrx::datum::array::Array<&core::ffi::c_str::CStr> */
) RETURNS INT /* i32 */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'exact_typmod_in_wrapper';
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
DROP FUNCTION IF EXISTS search_with_match_conjunction_boost(_field text, terms_to_tokenize boost);
CREATE OR REPLACE FUNCTION search_with_match_conjunction_boost(_field anyelement, terms_to_tokenize boost) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_conjunction_boost_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/andandand.rs:57
-- pg_search::api::operator::andandand::search_with_match_conjunction_boost
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction_boost",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=boost /* pg_search::api::operator::boost::BoostType */
    );
DROP FUNCTION IF EXISTS search_with_match_conjunction_fuzzy(_field text, terms_to_tokenize fuzzy);
CREATE OR REPLACE FUNCTION search_with_match_conjunction_fuzzy(_field anyelement, terms_to_tokenize fuzzy) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_conjunction_fuzzy_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/andandand.rs:65
-- pg_search::api::operator::andandand::search_with_match_conjunction_fuzzy
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction_fuzzy",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
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
DROP FUNCTION IF EXISTS search_with_match_disjunction_boost(_field text, terms_to_tokenize boost);
CREATE OR REPLACE FUNCTION search_with_match_disjunction_boost(_field anyelement, terms_to_tokenize boost) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_disjunction_boost_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/ororor.rs:56
-- pg_search::api::operator::ororor::search_with_match_disjunction_boost
CREATE OPERATOR pg_catalog.||| (
    PROCEDURE="search_with_match_disjunction_boost",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=boost /* pg_search::api::operator::boost::BoostType */
    );
DROP FUNCTION IF EXISTS search_with_match_disjunction_fuzzy(_field text, terms_to_tokenize fuzzy);
CREATE OR REPLACE FUNCTION search_with_match_disjunction_fuzzy(_field anyelement, terms_to_tokenize fuzzy) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_disjunction_fuzzy_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/ororor.rs:63
-- pg_search::api::operator::ororor::search_with_match_disjunction_fuzzy
CREATE OPERATOR pg_catalog.||| (
    PROCEDURE="search_with_match_disjunction_fuzzy",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
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
DROP FUNCTION IF EXISTS search_with_phrase_boost(_field text, terms_to_tokenize boost);
CREATE OR REPLACE FUNCTION search_with_phrase_boost(_field anyelement, terms_to_tokenize boost) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_phrase_boost_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/hashhashhash.rs:52
-- pg_search::api::operator::hashhashhash::search_with_phrase_boost
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase_boost",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=boost /* pg_search::api::operator::boost::BoostType */
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
DROP FUNCTION IF EXISTS search_with_term_boost(_field text, term boost);
CREATE OR REPLACE FUNCTION search_with_term_boost(_field anyelement, term boost) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_term_boost_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/eqeqeq.rs:48
-- pg_search::api::operator::eqeqeq::search_with_term_boost
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term_boost",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=boost /* pg_search::api::operator::boost::BoostType */
    );
DROP FUNCTION IF EXISTS search_with_term_fuzzy(_field text, term fuzzy);
CREATE OR REPLACE FUNCTION search_with_term_fuzzy(_field anyelement, term fuzzy) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_term_fuzzy_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/eqeqeq.rs:54
-- pg_search::api::operator::eqeqeq::search_with_term_fuzzy
CREATE OPERATOR pg_catalog.=== (
    PROCEDURE="search_with_term_fuzzy",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=fuzzy /* pg_search::api::operator::fuzzy::FuzzyType */
    );
DROP FUNCTION IF EXISTS search_with_phrase_slop(_field text, terms_to_tokenize slop);
CREATE OR REPLACE FUNCTION search_with_phrase_slop(_field anyelement, terms_to_tokenize slop) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_phrase_slop_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/hashhashhash.rs:60
-- pg_search::api::operator::hashhashhash::search_with_phrase_slop
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase_slop",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=slop /* pg_search::api::operator::slop::SlopType */
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
ALTER FUNCTION paradedb.search_with_match_disjunction_array SUPPORT paradedb.search_with_match_disjunction_support;
DROP FUNCTION IF EXISTS search_with_phrase_pdb_query(_field text, terms_to_tokenize pdb.query);
CREATE OR REPLACE FUNCTION search_with_phrase_pdb_query(_field anyelement, terms_to_tokenize pdb.query) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_phrase_pdb_query_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/hashhashhash.rs:44
-- pg_search::api::operator::hashhashhash::search_with_phrase_pdb_query
CREATE OPERATOR pg_catalog.### (
    PROCEDURE="search_with_phrase_pdb_query",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    );
ALTER FUNCTION paradedb.search_with_phrase_array SUPPORT paradedb.search_with_phrase_support;
DROP FUNCTION IF EXISTS search_with_match_conjunction_pdb_query(_field text, terms_to_tokenize pdb.query);
CREATE OR REPLACE FUNCTION search_with_match_conjunction_pdb_query(_field anyelement, terms_to_tokenize pdb.query) RETURNS bool AS 'MODULE_PATHNAME', 'search_with_match_conjunction_pdb_query_wrapper' COST 1000000000 IMMUTABLE LANGUAGE c PARALLEL SAFE STRICT;
-- pg_search/src/api/operator/andandand.rs:46
-- pg_search::api::operator::andandand::search_with_match_conjunction_pdb_query
CREATE OPERATOR pg_catalog.&&& (
    PROCEDURE="search_with_match_conjunction_pdb_query",
    LEFTARG=anyelement, /* pgrx::datum::anyelement::AnyElement */
    RIGHTARG=pdb.Query /* pg_search::query::pdb_query::pdb::Query */
    );
ALTER FUNCTION paradedb.search_with_match_conjunction_array SUPPORT paradedb.search_with_match_conjunction_support;
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


CREATE TYPE pdb.exact;
CREATE OR REPLACE FUNCTION pdb.exact_in(cstring) RETURNS pdb.exact AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.exact_out(pdb.exact) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.exact_send(pdb.exact) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.exact_recv(internal) RETURNS pdb.exact AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.exact (
                          INPUT = pdb.exact_in,
                          OUTPUT = pdb.exact_out,
                          SEND = pdb.exact_send,
                          RECEIVE = pdb.exact_recv,
                          COLLATABLE = true,
                          CATEGORY = 't', -- 't' is for tokenizer
                          PREFERRED = false,
                          LIKE = text
                      );
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:329
-- requires:
--   exact_typmod_in
--   exact_definition


ALTER TYPE pdb.exact SET (TYPMOD_IN = exact_typmod_in);
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


CREATE TYPE pdb.regex;
CREATE OR REPLACE FUNCTION pdb.regex_in(cstring) RETURNS pdb.regex AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.regex_out(pdb.regex) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.regex_send(pdb.regex) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.regex_recv(internal) RETURNS pdb.regex AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.regex (
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

ALTER TYPE pdb.regex SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:214
-- pg_search::api::tokenizers::definitions::pdb::tokenize_exact
CREATE  FUNCTION pdb."tokenize_exact"(
    "s" pdb.exact /* pg_search::api::tokenizers::definitions::pdb::Exact */
) RETURNS TEXT[] /* alloc::vec::Vec<alloc::string::String> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'tokenize_exact_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:214
-- requires:
--   exact_definition
--   tokenize_exact

CREATE CAST (pdb.exact AS TEXT[]) WITH FUNCTION pdb.tokenize_exact AS IMPLICIT;
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
    "s" pdb.regex /* pg_search::api::tokenizers::definitions::pdb::Regex */
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

CREATE CAST (pdb.regex AS TEXT[]) WITH FUNCTION pdb.tokenize_regex AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- pg_search::api::tokenizers::definitions::pdb::json_to_regex
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."json_to_regex"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.regex /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Regex> */
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
-- pg_search::api::tokenizers::definitions::pdb::json_to_exact
-- requires:
--   tokenize_exact
CREATE  FUNCTION pdb."json_to_exact"(
    "json" json /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::Json> */
) RETURNS pdb.exact /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Exact> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'json_to_exact_wrapper';
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
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_exact
-- requires:
--   tokenize_exact
CREATE  FUNCTION pdb."jsonb_to_exact"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.exact /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Exact> */
    IMMUTABLE STRICT PARALLEL SAFE
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'jsonb_to_exact_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:214
-- requires:
--   exact_definition
--   json_to_exact
--   jsonb_to_exact


CREATE CAST (json AS pdb.exact) WITH FUNCTION pdb.json_to_exact AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.exact) WITH FUNCTION pdb.jsonb_to_exact AS ASSIGNMENT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:297
-- pg_search::api::tokenizers::definitions::pdb::jsonb_to_regex
-- requires:
--   tokenize_regex
CREATE  FUNCTION pdb."jsonb_to_regex"(
    "jsonb" jsonb /* pg_search::api::tokenizers::GenericTypeWrapper<pgrx::datum::json::JsonB> */
) RETURNS pdb.regex /* pg_search::api::tokenizers::GenericTypeWrapper<pg_search::api::tokenizers::definitions::pdb::Regex> */
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


CREATE CAST (json AS pdb.regex) WITH FUNCTION pdb.json_to_regex AS ASSIGNMENT;
CREATE CAST (jsonb AS pdb.regex) WITH FUNCTION pdb.jsonb_to_regex AS ASSIGNMENT;
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

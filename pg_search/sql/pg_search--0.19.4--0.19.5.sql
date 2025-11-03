/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/score.rs:44
-- pg_search::postgres::customscan::pdbscan::projections::score::score
CREATE OR REPLACE FUNCTION paradedb.score(
	"_relation_reference" anyelement /* pgrx::datum::anyelement::AnyElement */
) RETURNS real /* core::option::Option<f32> */
STRICT STABLE PARALLEL SAFE  COST 1
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'paradedb_score_from_relation_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/score.rs:49
-- requires:
--   paradedb_score_from_relation
--   placeholder_support
    ALTER FUNCTION paradedb.score SUPPORT paradedb.placeholder_support;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:236
-- pg_search::postgres::customscan::pdbscan::projections::snippet::snippet
CREATE OR REPLACE FUNCTION paradedb.snippet(
	"field" anyelement, /* pgrx::datum::anyelement::AnyElement */
	"start_tag" TEXT DEFAULT '<b>', /* alloc::string::String */
	"end_tag" TEXT DEFAULT '</b>', /* alloc::string::String */
	"max_num_chars" INT DEFAULT 150, /* i32 */
	"limit" INT DEFAULT NULL, /* core::option::Option<i32> */
	"offset" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS TEXT /* core::option::Option<alloc::string::String> */
STABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'paradedb_snippet_from_relation_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:257
-- requires:
--   paradedb_snippet_from_relation
--   placeholder_support
    ALTER FUNCTION paradedb.snippet SUPPORT paradedb.placeholder_support;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:236
-- pg_search::postgres::customscan::pdbscan::projections::snippet::snippet_positions
CREATE OR REPLACE FUNCTION paradedb.snippet_positions(
	"field" anyelement, /* pgrx::datum::anyelement::AnyElement */
	"limit" INT DEFAULT NULL, /* core::option::Option<i32> */
	"offset" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS INT[][] /* core::option::Option<alloc::vec::Vec<alloc::vec::Vec<i32>>> */
STABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'paradedb_snippet_positions_from_relation_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:265
-- requires:
--   paradedb_snippet_positions_from_relation
--   placeholder_support
    ALTER FUNCTION paradedb.snippet_positions SUPPORT paradedb.placeholder_support;
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:236
-- pg_search::postgres::customscan::pdbscan::projections::snippet::snippets
CREATE OR REPLACE FUNCTION paradedb.snippets(
	"field" anyelement, /* pgrx::datum::anyelement::AnyElement */
	"start_tag" TEXT DEFAULT '<b>', /* alloc::string::String */
	"end_tag" TEXT DEFAULT '</b>', /* alloc::string::String */
	"max_num_chars" INT DEFAULT 150, /* i32 */
	"limit" INT DEFAULT NULL, /* core::option::Option<i32> */
	"offset" INT DEFAULT NULL, /* core::option::Option<i32> */
	"sort_by" TEXT DEFAULT 'score' /* alloc::string::String */
) RETURNS TEXT[] /* core::option::Option<alloc::vec::Vec<alloc::string::String>> */
STABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'paradedb_snippets_from_relation_wrapper';
-- pg_search/src/api/builder_fns/pdb.rs:149
-- pg_search::api::builder_fns::pdb::pdb::_f30e8accdb684eec9e64d6ac49e3167e::parse
CREATE  FUNCTION "parse"(
	"field" FieldName, /* pg_search::api::FieldName */
	"query_string" TEXT, /* alloc::string::String */
	"lenient" bool DEFAULT NULL, /* core::option::Option<bool> */
	"conjunction_mode" bool DEFAULT NULL /* core::option::Option<bool> */
) RETURNS SearchQueryInput /* pg_search::query::SearchQueryInput */
IMMUTABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'parse_query_bfn_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:149
-- pg_search::api::builder_fns::pdb::pdb::parse
CREATE  FUNCTION pdb."parse"(
	"query_string" TEXT, /* alloc::string::String */
	"lenient" bool DEFAULT NULL, /* core::option::Option<bool> */
	"conjunction_mode" bool DEFAULT NULL /* core::option::Option<bool> */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
IMMUTABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'parse_query_wrapper';

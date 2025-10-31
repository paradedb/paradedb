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

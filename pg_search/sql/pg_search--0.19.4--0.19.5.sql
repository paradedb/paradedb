DROP FUNCTION IF EXISTS pdb.snippet(field anyelement, start_tag text, end_tag text, max_num_chars pg_catalog.int4, "limit" pg_catalog.int4, "offset" pg_catalog.int4);
DROP FUNCTION IF EXISTS pdb.snippet_positions(field anyelement, "limit" pg_catalog.int4, "offset" pg_catalog.int4);
DROP FUNCTION IF EXISTS pdb.snippets(field anyelement, start_tag text, end_tag text, max_num_chars pg_catalog.int4, "limit" pg_catalog.int4, "offset" pg_catalog.int4, sort_by text);
DROP FUNCTION IF EXISTS pdb.score(_relation_reference anyelement);

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

/* pg_search::postgres::customscan::pdbscan::projections::score::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/score.rs:28
-- pg_search::postgres::customscan::pdbscan::projections::score::pdb::score
CREATE  FUNCTION pdb."score"(
	"_relation_reference" anyelement /* pgrx::datum::anyelement::AnyElement */
) RETURNS real /* core::option::Option<f32> */
STRICT STABLE PARALLEL SAFE  COST 1
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'pdb_score_from_relation_wrapper';
/* pg_search::postgres::customscan::pdbscan::projections::snippet::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:231
-- pg_search::postgres::customscan::pdbscan::projections::snippet::pdb::snippet_positions
CREATE  FUNCTION pdb."snippet_positions"(
	"field" anyelement, /* pgrx::datum::anyelement::AnyElement */
	"limit" INT DEFAULT NULL, /* core::option::Option<i32> */
	"offset" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS INT[][] /* core::option::Option<alloc::vec::Vec<alloc::vec::Vec<i32>>> */
STABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'pdb_snippet_positions_from_relation_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:231
-- pg_search::postgres::customscan::pdbscan::projections::snippet::pdb::snippets
CREATE  FUNCTION pdb."snippets"(
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
AS 'MODULE_PATHNAME', 'pdb_snippets_from_relation_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:231
-- pg_search::postgres::customscan::pdbscan::projections::snippet::pdb::snippet
CREATE  FUNCTION pdb."snippet"(
	"field" anyelement, /* pgrx::datum::anyelement::AnyElement */
	"start_tag" TEXT DEFAULT '<b>', /* alloc::string::String */
	"end_tag" TEXT DEFAULT '</b>', /* alloc::string::String */
	"max_num_chars" INT DEFAULT 150, /* i32 */
	"limit" INT DEFAULT NULL, /* core::option::Option<i32> */
	"offset" INT DEFAULT NULL /* core::option::Option<i32> */
) RETURNS TEXT /* core::option::Option<alloc::string::String> */
STABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'pdb_snippet_from_relation_wrapper';

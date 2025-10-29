/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/postgres/customscan/pdbscan/projections/snippet.rs:204
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
AS 'MODULE_PATHNAME', 'snippets_from_relation_wrapper';

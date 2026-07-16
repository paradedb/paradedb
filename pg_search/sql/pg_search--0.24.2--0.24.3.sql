\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.24.3'" to load this file. \quit

-- Expose the pg_search version stamped into an index's metadata page at build time.
CREATE FUNCTION "index_created_by"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS TEXT /* core::option::Option<alloc::string::String> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'index_created_by_wrapper';

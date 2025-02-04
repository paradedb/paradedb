\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.15.2'" to load this file. \quit

-- pg_search::bootstrap::create_bm25::version_info
CREATE  FUNCTION "version_info"() RETURNS TABLE (
	"version" TEXT,  /* alloc::string::String */
	"githash" TEXT,  /* alloc::string::String */
	"build_mode" TEXT  /* alloc::string::String */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'version_info_wrapper';

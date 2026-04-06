/* pg_search::api::admin::pdb */
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/admin.rs:1508
-- pg_search::api::admin::pdb::indexes
CREATE  FUNCTION pdb."indexes"() RETURNS TABLE (
	"schemaname" TEXT,  /* alloc::string::String */
	"tablename" TEXT,  /* alloc::string::String */
	"indexname" TEXT,  /* alloc::string::String */
	"indexrelid" oid,  /* pgrx_pg_sys::submodules::oids::Oid */
	"num_segments" INT,  /* i32 */
	"total_docs" bigint  /* i64 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'indexes_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/admin.rs:1425
-- pg_search::api::admin::pdb::index_segments
CREATE  FUNCTION pdb."index_segments"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
	"partition_name" TEXT,  /* alloc::string::String */
	"segment_idx" INT,  /* i32 */
	"segment_id" TEXT,  /* alloc::string::String */
	"num_docs" bigint,  /* i64 */
	"num_deleted" bigint,  /* i64 */
	"max_doc" bigint  /* i64 */
)
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'index_segments_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/admin.rs:1645
-- pg_search::api::admin::pdb::verify_all_indexes
CREATE  FUNCTION pdb."verify_all_indexes"(
	"schema_pattern" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"index_pattern" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"heapallindexed" bool DEFAULT false, /* bool */
	"sample_rate" double precision DEFAULT NULL, /* core::option::Option<f64> */
	"report_progress" bool DEFAULT false, /* bool */
	"on_error_stop" bool DEFAULT false /* bool */
) RETURNS TABLE (
	"schemaname" TEXT,  /* alloc::string::String */
	"indexname" TEXT,  /* alloc::string::String */
	"check_name" TEXT,  /* alloc::string::String */
	"passed" bool,  /* bool */
	"details" TEXT  /* core::option::Option<alloc::string::String> */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'verify_all_indexes_wrapper';
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/admin.rs:961
-- pg_search::api::admin::pdb::verify_index
CREATE  FUNCTION pdb."verify_index"(
	"index" regclass, /* pgrx::rel::PgRelation */
	"heapallindexed" bool DEFAULT false, /* bool */
	"sample_rate" double precision DEFAULT NULL, /* core::option::Option<f64> */
	"report_progress" bool DEFAULT false, /* bool */
	"verbose" bool DEFAULT false, /* bool */
	"on_error_stop" bool DEFAULT false, /* bool */
	"segment_ids" INT[] DEFAULT NULL /* core::option::Option<alloc::vec::Vec<i32>> */
) RETURNS TABLE (
	"check_name" TEXT,  /* alloc::string::String */
	"passed" bool,  /* bool */
	"details" TEXT  /* core::option::Option<alloc::string::String> */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'verify_index_wrapper';
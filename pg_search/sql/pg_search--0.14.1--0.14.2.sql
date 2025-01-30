-- pg_search/src/bootstrap/create_bm25.rs:404
-- pg_search::bootstrap::create_bm25::find_ctid
CREATE  FUNCTION "find_ctid"(
    "index" regclass, /* pgrx::rel::PgRelation */
    "ctid" tid /* pgrx_pg_sys::include::pg13::ItemPointerData */
) RETURNS TEXT[] /* core::result::Result<core::option::Option<alloc::vec::Vec<alloc::string::String>>, anyhow::Error> */
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'find_ctid_wrapper';

DROP FUNCTION IF EXISTS index_info(index regclass);
CREATE OR REPLACE FUNCTION index_info(index regclass, show_invisible bool DEFAULT false)
    RETURNS TABLE(visible bool, recyclable bool, xmin pg_catalog."numeric", xmax pg_catalog."numeric", segno text, byte_size pg_catalog."numeric", num_docs pg_catalog."numeric", num_deleted pg_catalog."numeric", termdict_bytes pg_catalog."numeric", postings_bytes pg_catalog."numeric", positions_bytes pg_catalog."numeric", fast_fields_bytes pg_catalog."numeric", fieldnorms_bytes pg_catalog."numeric", store_bytes pg_catalog."numeric", deletes_bytes pg_catalog."numeric")
    AS 'MODULE_PATHNAME', 'index_info_wrapper'
    LANGUAGE c STRICT;
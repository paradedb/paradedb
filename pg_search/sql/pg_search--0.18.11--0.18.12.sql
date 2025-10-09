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

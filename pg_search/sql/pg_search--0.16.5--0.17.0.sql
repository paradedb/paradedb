CREATE  FUNCTION "fsm_info"(
    "index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
                    "fsm_blockno" NUMERIC,  /* pgrx::datum::numeric::AnyNumeric */
                    "free_blockno" NUMERIC  /* pgrx::datum::numeric::AnyNumeric */
                )
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'fsm_info_wrapper';

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/bootstrap/create_bm25.rs:62
-- pg_search::bootstrap::create_bm25::background_layer_sizes
CREATE  FUNCTION "background_layer_sizes"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS NUMERIC[] /* alloc::vec::Vec<pgrx::datum::numeric::AnyNumeric> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'background_layer_sizes_wrapper';
DROP FUNCTION IF EXISTS force_merge(index regclass, oversized_layer_size_pretty text);
CREATE OR REPLACE FUNCTION force_merge(_index regclass, _oversized_layer_size_pretty text) RETURNS TABLE(new_segments pg_catalog.int8, merged_segments pg_catalog.int8) AS 'MODULE_PATHNAME', 'force_merge_pretty_bytes_wrapper' LANGUAGE c STRICT;
DROP FUNCTION IF EXISTS force_merge(index regclass, oversized_layer_size_bytes pg_catalog.int8);
CREATE OR REPLACE FUNCTION force_merge(_index regclass, _oversized_layer_size_bytes pg_catalog.int8) RETURNS TABLE(new_segments pg_catalog.int8, merged_segments pg_catalog.int8) AS 'MODULE_PATHNAME', 'force_merge_raw_bytes_wrapper' LANGUAGE c STRICT;

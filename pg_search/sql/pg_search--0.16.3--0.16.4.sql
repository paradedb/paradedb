CREATE  FUNCTION "fsm_info"(
    "index" regclass /* pgrx::rel::PgRelation */
) RETURNS TABLE (
                    "fsm_blockno" NUMERIC,  /* pgrx::datum::numeric::AnyNumeric */
                    "free_blockno" NUMERIC  /* pgrx::datum::numeric::AnyNumeric */
                )
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'fsm_info_wrapper';
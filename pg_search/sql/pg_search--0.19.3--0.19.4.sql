/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:92
-- pg_search::api::aggregate::agg_placeholder_agg_placeholder_finalize
CREATE  FUNCTION "agg_placeholder_agg_placeholder_finalize"(
	"this" internal /* pgrx::datum::internal::Internal */
) RETURNS jsonb /* pgrx::datum::json::JsonB */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_finalize_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:92
-- pg_search::api::aggregate::agg_placeholder_agg_placeholder_state
CREATE  FUNCTION "agg_placeholder_agg_placeholder_state"(
	"this" internal, /* pgrx::datum::internal::Internal */
	"arg_one" jsonb /* pgrx::datum::json::JsonB */
) RETURNS internal /* pgrx::datum::internal::Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_state_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/window_aggregate.rs:52
-- pg_search::api::window_aggregate::window_agg
CREATE  FUNCTION "window_agg"(
	"window_aggregate_json" TEXT /* &str */
) RETURNS bigint /* i64 */
STRICT VOLATILE PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'window_agg_placeholder_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:92
-- pg_search::api::aggregate::AggPlaceholder
CREATE AGGREGATE agg (
	jsonb /* pgrx::datum::json::JsonB */
)
(
	SFUNC = "agg_placeholder_agg_placeholder_state", /* pg_search::api::aggregate::AggPlaceholder::state */
	STYPE = internal, /* pgrx::datum::internal::Internal */
	FINALFUNC = "agg_placeholder_agg_placeholder_finalize" /* pg_search::api::aggregate::AggPlaceholder::final */
);
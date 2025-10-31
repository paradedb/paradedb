/* pg_search::api::aggregate::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::agg_placeholder_agg_placeholder_state
CREATE  FUNCTION pdb."agg_placeholder_agg_placeholder_state"(
	"this" internal, /* pgrx::datum::internal::Internal */
	"arg_one" jsonb /* pgrx::datum::json::JsonB */
) RETURNS internal /* pgrx::datum::internal::Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_state_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::agg_placeholder_agg_placeholder_finalize
CREATE  FUNCTION pdb."agg_placeholder_agg_placeholder_finalize"(
	"this" internal /* pgrx::datum::internal::Internal */
) RETURNS jsonb /* pgrx::datum::json::JsonB */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_agg_placeholder_finalize_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::AggPlaceholder
CREATE AGGREGATE pdb.agg (
	jsonb /* pgrx::datum::json::JsonB */
)
(
	SFUNC = pdb."agg_placeholder_agg_placeholder_state", /* pg_search::api::aggregate::pdb::AggPlaceholder::state */
	STYPE = internal, /* pgrx::datum::internal::Internal */
	FINALFUNC = pdb."agg_placeholder_agg_placeholder_finalize" /* pg_search::api::aggregate::pdb::AggPlaceholder::final */
);
/* pg_search::api::window_aggregate::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/window_aggregate.rs:57
-- pg_search::api::window_aggregate::pdb::window_agg
CREATE  FUNCTION pdb."window_agg"(
	"window_aggregate_json" TEXT /* &str */
) RETURNS bigint /* i64 */
STRICT VOLATILE PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'window_agg_placeholder_wrapper';
/* pg_search::api::aggregate::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:136
-- pg_search::api::aggregate::pdb::agg_fn
CREATE  FUNCTION pdb."agg_fn"(
	"_agg_name" TEXT /* &str */
) RETURNS bigint /* i64 */
STRICT VOLATILE PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_fn_placeholder_wrapper';

/* pg_search::api::aggregate::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state
CREATE  FUNCTION pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state"(
	"this" internal, /* pgrx::datum::internal::Internal */
	"arg_one" jsonb /* pgrx::datum::json::JsonB */
) RETURNS internal /* pgrx::datum::internal::Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize
CREATE  FUNCTION pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize"(
	"this" internal /* pgrx::datum::internal::Internal */
) RETURNS jsonb /* pgrx::datum::json::JsonB */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:94
-- pg_search::api::aggregate::pdb::AggPlaceholderWithMVCC
CREATE AGGREGATE pdb.agg (
	jsonb /* pgrx::datum::json::JsonB */
)
(
	SFUNC = pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state", /* pg_search::api::aggregate::pdb::AggPlaceholderWithMVCC::state */
	STYPE = internal, /* pgrx::datum::internal::Internal */
	FINALFUNC = pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize" /* pg_search::api::aggregate::pdb::AggPlaceholderWithMVCC::final */
);
/* pg_search::api::aggregate::pdb */
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:150
-- pg_search::api::aggregate::pdb::agg_fn
CREATE  FUNCTION pdb."agg_fn"(
	"_agg_name" TEXT, /* &str */
	"_solve_mvcc" bool /* bool */
) RETURNS jsonb /* pgrx::datum::json::JsonB */
STRICT VOLATILE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_fn_placeholder_with_mvcc_wrapper';

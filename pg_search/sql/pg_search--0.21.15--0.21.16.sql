-- pg_search/src/api/aggregate.rs:195
-- pg_search::api::aggregate::pdb::agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state
DROP FUNCTION IF EXISTS pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state"(internal, jsonb, bool);
CREATE  FUNCTION pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state"(
	"this" internal, /* pgrx::datum::internal::Internal */
	"arg_one" jsonb, /* pgrx::datum::json::JsonB */
	"arg_two" bool /* bool */
) RETURNS internal /* pgrx::datum::internal::Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state_wrapper';

-- pg_search/src/api/aggregate.rs:195
-- pg_search::api::aggregate::pdb::agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize
DROP FUNCTION IF EXISTS pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize"(internal);
CREATE  FUNCTION pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize"(
	"this" internal /* pgrx::datum::internal::Internal */
) RETURNS jsonb /* pgrx::datum::json::JsonB */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize_wrapper';

-- pg_search/src/api/aggregate.rs:195
-- pg_search::api::aggregate::pdb::AggPlaceholderWithMvcc
DROP AGGREGATE IF EXISTS pdb.agg(jsonb, bool);
CREATE AGGREGATE pdb.agg (
	jsonb, /* pgrx::datum::json::JsonB */
	bool /* bool */
)
(
	SFUNC = pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_state", /* pg_search::api::aggregate::pdb::AggPlaceholderWithMvcc::state */
	STYPE = internal, /* pgrx::datum::internal::Internal */
	FINALFUNC = pdb."agg_placeholder_with_mvcc_agg_placeholder_with_mvcc_finalize" /* pg_search::api::aggregate::pdb::AggPlaceholderWithMvcc::final */
);

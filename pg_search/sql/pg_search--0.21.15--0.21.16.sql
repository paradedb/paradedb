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

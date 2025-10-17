/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:646
-- pg_search::api::builder_fns::pdb::pdb::term_set_agg_i_64_term_set_agg_i_64_finalize
CREATE  FUNCTION pdb."term_set_agg_i_64_term_set_agg_i_64_finalize"(
	"this" internal /* pgrx::datum::internal::Internal */
) RETURNS pdb.Query /* pg_search::query::pdb_query::pdb::Query */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_agg_i_64_term_set_agg_i_64_finalize_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:646
-- pg_search::api::builder_fns::pdb::pdb::term_set_agg_i_64_term_set_agg_i_64_combine
CREATE  FUNCTION pdb."term_set_agg_i_64_term_set_agg_i_64_combine"(
	"this" internal, /* pgrx::datum::internal::Internal */
	"v" internal /* pgrx::datum::internal::Internal */
) RETURNS internal /* pgrx::datum::internal::Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_agg_i_64_term_set_agg_i_64_combine_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:646
-- pg_search::api::builder_fns::pdb::pdb::term_set_agg_i_64_term_set_agg_i_64_state
CREATE  FUNCTION pdb."term_set_agg_i_64_term_set_agg_i_64_state"(
	"this" internal, /* pgrx::datum::internal::Internal */
	"arg_one" bigint /* i64 */
) RETURNS internal /* pgrx::datum::internal::Internal */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'term_set_agg_i_64_term_set_agg_i_64_state_wrapper';
/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/builder_fns/pdb.rs:646
-- pg_search::api::builder_fns::pdb::pdb::TermSetAggI64
CREATE AGGREGATE pdb.term_set (
	bigint /* i64 */
)
(
	SFUNC = pdb."term_set_agg_i_64_term_set_agg_i_64_state", /* pg_search::api::builder_fns::pdb::pdb::TermSetAggI64::state */
	STYPE = internal, /* pgrx::datum::internal::Internal */
	FINALFUNC = pdb."term_set_agg_i_64_term_set_agg_i_64_finalize", /* pg_search::api::builder_fns::pdb::pdb::TermSetAggI64::final */
	COMBINEFUNC = pdb."term_set_agg_i_64_term_set_agg_i_64_combine" /* pg_search::api::builder_fns::pdb::pdb::TermSetAggI64::combine */
);

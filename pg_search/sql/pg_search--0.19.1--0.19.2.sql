\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.19.2'" to load this file. \quit

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/window_function.rs:28
-- pg_search::api::window_function::window_func
CREATE OR REPLACE FUNCTION "window_func"(
	"window_aggregate_json" TEXT /* &str */
) RETURNS bigint /* i64 */
STRICT VOLATILE PARALLEL SAFE 
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'window_func_placeholder_wrapper';

CREATE OR REPLACE AGGREGATE "agg"(JSONB) (
    SFUNC = "agg_sfunc",
    STYPE = internal,
    FINALFUNC = "agg_finalfunc",
    PARALLEL = SAFE
);

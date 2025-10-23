CREATE  FUNCTION "combined_layer_sizes"(
	"index" regclass /* pgrx::rel::PgRelation */
) RETURNS NUMERIC[] /* alloc::vec::Vec<pgrx::datum::numeric::AnyNumeric> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'combined_layer_sizes_wrapper';

CREATE VIEW pdb.index_layer_info as
select relname::text,
       layer_size,
       low,
       high,
       byte_size,
       case when segments = ARRAY [NULL] then 0 else count end       as count,
       case when segments = ARRAY [NULL] then NULL else segments end as segments
from (select relname,
             coalesce(pg_size_pretty(case when low = 0 then null else low end), '') || '..' ||
             coalesce(pg_size_pretty(case when high = 9223372036854775807 then null else high end), '') as layer_size,
             count(*),
             coalesce(sum(byte_size), 0)                                                                as byte_size,
             min(low)                                                                                   as low,
             max(high)                                                                                  as high,
             array_agg(segno)                                                                           as segments
      from (with indexes as (select oid::regclass as relname
                             from pg_class
                             where relam = (select oid from pg_am where amname = 'bm25')),
                 segments as (select relname, index_info.*
                              from indexes
                                       inner join paradedb.index_info(indexes.relname, true) on true),
                 layer_sizes as (select relname, coalesce(lead(unnest) over (), 0) low, unnest as high
                                 from indexes
                                          inner join lateral (select unnest(0 || paradedb.combined_layer_sizes(indexes.relname) || 9223372036854775807)
                                                              order by 1 desc) x on true)
            select layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size
            from layer_sizes
                     left join segments on layer_sizes.relname = segments.relname and
                                           (byte_size * 1.33)::bigint between low and high) x
      where low < high
      group by relname, low, high
      order by relname, low desc) x;

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

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:58
-- pg_search::api::aggregate::agg_sfunc
CREATE OR REPLACE FUNCTION "agg_sfunc"(
    internal,
    jsonb
)  RETURNS internal
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_sfunc_placeholder_wrapper';

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/api/aggregate.rs:89
-- pg_search::api::aggregate::agg_finalfunc
CREATE OR REPLACE FUNCTION "agg_finalfunc"(
    internal,
)  RETURNS jsonb
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'agg_finalfunc_placeholder_wrapper';

/* </end connected objects> */
/* <begin connected objects> */
-- pg_search/src/lib.rs:63
-- pg_search::lib::extension_sql!
CREATE OR REPLACE AGGREGATE "agg"(JSONB) (
    SFUNC = "agg_sfunc",
    STYPE = internal,
    FINALFUNC = "agg_finalfunc",
    PARALLEL = SAFE
);
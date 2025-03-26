drop view if exists paradedb.index_layer_info;
create view paradedb.index_layer_info as
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
                                          inner join lateral (select unnest(0 || paradedb.layer_sizes(indexes.relname) || 9223372036854775807)
                                                              order by 1 desc) x on true)
            select layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size
            from layer_sizes
                     left join segments on layer_sizes.relname = segments.relname and
                                           (byte_size * 1.33)::bigint between low and high) x
      where low < high
      group by relname, low, high
      order by relname, low desc) x;

-- pg_search/src/bootstrap/create_bm25.rs:424
-- pg_search::bootstrap::create_bm25::force_merge
CREATE  FUNCTION "force_merge"(
    "index" regclass, /* pgrx::rel::PgRelation */
    "oversized_layer_size_pretty" TEXT /* alloc::string::String */
) RETURNS TABLE (
                    "new_segments" bigint,  /* i64 */
                    "merged_segments" bigint  /* i64 */
                )
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'force_merge_pretty_bytes_wrapper';

-- pg_search/src/bootstrap/create_bm25.rs:441
-- pg_search::bootstrap::create_bm25::force_merge
CREATE  FUNCTION "force_merge"(
    "index" regclass, /* pgrx::rel::PgRelation */
    "oversized_layer_size_bytes" bigint /* i64 */
) RETURNS TABLE (
                    "new_segments" bigint,  /* i64 */
                    "merged_segments" bigint  /* i64 */
                )
    STRICT
    LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'force_merge_raw_bytes_wrapper';

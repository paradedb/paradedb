DROP FUNCTION IF EXISTS index_info(index regclass, show_invisible bool) CASCADE;
CREATE OR REPLACE FUNCTION index_info(index regclass, show_invisible bool DEFAULT false)
    RETURNS TABLE
            (
                index_name        text,
                visible           bool,
                recyclable        bool,
                xmax              xid,
                segno             text,
                byte_size         pg_catalog."numeric",
                num_docs          pg_catalog."numeric",
                num_deleted       pg_catalog."numeric",
                termdict_bytes    pg_catalog."numeric",
                postings_bytes    pg_catalog."numeric",
                positions_bytes   pg_catalog."numeric",
                fast_fields_bytes pg_catalog."numeric",
                fieldnorms_bytes  pg_catalog."numeric",
                store_bytes       pg_catalog."numeric",
                deletes_bytes     pg_catalog."numeric"
            )
AS
'MODULE_PATHNAME',
'index_info_wrapper' LANGUAGE c STRICT;

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

DROP FUNCTION IF EXISTS merge_info(index regclass);
CREATE OR REPLACE FUNCTION merge_info(index regclass)
    RETURNS TABLE
            (
                index_name text,
                pid        pg_catalog.int4,
                xmin       xid,
                segno      text
            )
AS
'MODULE_PATHNAME',
'merge_info_wrapper' LANGUAGE c STRICT;

ALTER FUNCTION paradedb.jsonb_to_searchqueryinput IMMUTABLE STRICT PARALLEL SAFE;

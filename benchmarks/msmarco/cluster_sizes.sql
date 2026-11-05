CREATE TEMP TABLE cluster_sizes AS
SELECT cluster_id,
       count(*) AS posting_pages,
       sum(entry_count)::bigint AS entries,
       sum(dead_count)::bigint AS dead_entries,
       sum(CASE WHEN tombstoned THEN 0 ELSE entry_count - dead_count END)::bigint AS live_entries
FROM prism_posting_pages(:'index_name'::regclass)
GROUP BY cluster_id;

\o :sizes_file
COPY (SELECT * FROM cluster_sizes ORDER BY cluster_id) TO STDOUT WITH (FORMAT csv, HEADER true);

\o :summary_file
\pset format unaligned
\pset tuples_only on
SELECT json_build_object(
    'index', :'index_name',
    'cluster_count', count(*),
    'posting_pages', sum(posting_pages),
    'total_entries', sum(entries),
    'dead_entries', sum(dead_entries),
    'live_entries', sum(live_entries),
    'empty_clusters', count(*) FILTER (WHERE live_entries = 0),
    'min', min(live_entries),
    'mean', avg(live_entries),
    'p50', percentile_disc(0.50) WITHIN GROUP (ORDER BY live_entries),
    'p95', percentile_disc(0.95) WITHIN GROUP (ORDER BY live_entries),
    'p99', percentile_disc(0.99) WITHIN GROUP (ORDER BY live_entries),
    'p999', percentile_disc(0.999) WITHIN GROUP (ORDER BY live_entries),
    'max', max(live_entries),
    'stddev', stddev_pop(live_entries),
    'max_to_mean', max(live_entries) / nullif(avg(live_entries), 0)
)
FROM cluster_sizes;
\o

WITH segments AS MATERIALIZED (
    SELECT * FROM paradedb.index_info('idxtest', true)
    UNION ALL
    SELECT * FROM paradedb.index_info('idxupserts', true)
), counts AS (
    SELECT index_name,
           count(*) FILTER (WHERE visible AND NOT mutable) AS immutable,
           count(*) FILTER (WHERE visible AND mutable) AS mutable,
           count(*) FILTER (WHERE NOT visible) AS invisible,
           sum(byte_size) FILTER (WHERE visible AND NOT mutable) AS immutable_bytes,
           sum(num_docs) FILTER (WHERE visible) AS docs,
           sum(num_deleted) FILTER (WHERE visible) AS deleted
    FROM segments GROUP BY index_name
)
SELECT json_build_object(
    'sampled_at', clock_timestamp(),
    'counts', (SELECT json_agg(counts) FROM counts),
    'segments', (SELECT json_agg(s) FROM (
        SELECT index_name, segno, visible, mutable, recyclable, byte_size, num_docs, num_deleted
        FROM segments
    ) s),
    'mergers', (SELECT json_agg(a) FROM (
        SELECT pid, backend_type, state, wait_event_type, wait_event, xact_start, query_start
        FROM pg_stat_activity
        WHERE datname = current_database() AND position('background merger' IN backend_type) > 0
    ) a),
    'transactions', (SELECT xact_commit FROM pg_stat_database WHERE datname = current_database())
);

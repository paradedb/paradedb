-- Builds a partitioned index where one partition has a column with a dictionary over 4 GiB.
-- One of the two tenants has 99.9% of the rows, so `partition_by = 'tenant_id'` makes that partition.
--
-- psql variables:
--   top_level (0)       0 indexes the `jsonb` column, so the long value is a JSON key column.
--                       1 indexes the value as a top-level text field.
--   rows (6600000), value_bytes (750), mwm ('1GB'), workers (2)
\set ON_ERROR_STOP on
\if :{?top_level} \else \set top_level 0 \endif
\if :{?rows} \else \set rows 6600000 \endif
\if :{?value_bytes} \else \set value_bytes 750 \endif
\if :{?mwm} \else \set mwm '1GB' \endif
\if :{?workers} \else \set workers 2 \endif

CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
DROP TABLE IF EXISTS big_entries;

-- Unlogged, so that neither the table nor the index writes WAL.
CREATE UNLOGGED TABLE big_entries (
    id uuid NOT NULL,
    tenant_id uuid NOT NULL,
    metadata jsonb NOT NULL
);

-- Random bytes in base64 do not compress below 75%, so 750 bytes take about 760 bytes in a
-- dictionary.
INSERT INTO big_entries
SELECT gen_random_uuid(),
       CASE WHEN i % 1000 = 0 THEN '00000000-0000-4000-8000-000000000001'::uuid
            ELSE '00000000-0000-4000-8000-000000000002'::uuid END,
       jsonb_build_object('key1', translate(encode(gen_random_bytes(:value_bytes), 'base64'), E'\n', ''))
FROM generate_series(1, :rows) i;

ANALYZE big_entries;
SELECT count(*) AS rows, round(avg(length(metadata::text))) AS avg_metadata_len,
       pg_size_pretty(pg_table_size('big_entries')) AS table_size
FROM big_entries;

SET maintenance_work_mem = :'mwm';
SET max_parallel_maintenance_workers = :workers;
-- The server log then has the partition boundaries and one line per flush and per merge.
SET log_min_messages = debug1;
\timing on

\if :top_level
CREATE INDEX CONCURRENTLY big_entries_idx ON big_entries
USING bm25 (id, ((metadata->>'key1')::pdb.literal('alias=payload')), tenant_id)
WITH (partition_by = 'tenant_id', target_segment_count = 4);
\else
CREATE INDEX CONCURRENTLY big_entries_idx ON big_entries
USING bm25 (id, (metadata::pdb.literal_normalized), tenant_id)
WITH (partition_by = 'tenant_id', target_segment_count = 4);
\endif

\timing off
RESET log_min_messages;
SELECT segno, num_docs, pg_size_pretty(byte_size) AS size, pg_size_pretty(columnar_bytes) AS columnar
FROM paradedb.index_info('big_entries_idx') ORDER BY num_docs DESC;

\if :top_level
\else
-- A grouped aggregate reads the column of `key1`, which the build does not open.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT length(v) AS value_len, n
FROM (SELECT metadata->>'key1' AS v, count(*) AS n FROM big_entries
      WHERE id @@@ pdb.all() GROUP BY 1 ORDER BY 1 LIMIT 3) s;
SELECT length(v) AS value_len, n
FROM (SELECT metadata->>'key1' AS v, count(*) AS n FROM big_entries
      WHERE id @@@ pdb.all() GROUP BY 1 ORDER BY 1 LIMIT 3) s;
\endif

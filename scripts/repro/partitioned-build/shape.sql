-- Builds a wide partitioned index with a small memory budget, so that each large partition
-- flushes many segments and merges them in one merge.
--
-- psql variables: rows (2000000), mwm ('80MB'), workers (4),
--                 index_options (partition_by = 'tenant_id, ctid')
\set ON_ERROR_STOP on
\if :{?rows} \else \set rows 2000000 \endif
\if :{?mwm} \else \set mwm '80MB' \endif
\if :{?workers} \else \set workers 4 \endif
\if :{?index_options} \else \set index_options 'partition_by = ''tenant_id, ctid''' \endif

CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
DROP TABLE IF EXISTS entries;
DROP TABLE IF EXISTS tenants;

SELECT setseed(0.42);
-- The large tenant's id sorts second. That order gives the largest partitions.
CREATE TABLE tenants AS
SELECT ord,
       format('%s-0000-4000-8000-000000000000',
              lpad((CASE ord WHEN 1 THEN 2 WHEN 2 THEN 1 ELSE ord END)::text, 8, '0'))::uuid AS id,
       sum(share) OVER (ORDER BY ord) - share AS lo,
       sum(share) OVER (ORDER BY ord) AS hi
FROM (VALUES (1, 0.81), (2, 0.1574), (3, 0.0281), (4, 0.0009), (5, 0.0009),
             (6, 0.0009), (7, 0.0009), (8, 0.0009)) v(ord, share);

CREATE TABLE entries (
    id uuid NOT NULL,
    side text,
    metadata jsonb NOT NULL,
    state text,
    amount numeric,
    created_at timestamptz,
    removed_at timestamptz,
    removed_at_version bigint,
    effective_at timestamptz,
    account_id uuid,
    account_version bigint,
    settlement_id uuid,
    book_id uuid,
    transaction_id uuid,
    transaction_version bigint,
    live boolean,
    tenant_id uuid NOT NULL,
    pending_in numeric,
    pending_out numeric,
    posted_in numeric,
    posted_out numeric,
    synchronous boolean,
    reversed_by_id uuid,
    reverses_id uuid,
    updated_at timestamptz
);

-- The metadata is a flat object of short strings. Most values are distinct, and one row in six
-- has an empty object.
INSERT INTO entries
SELECT gen_random_uuid(),
       (ARRAY['in', 'out'])[1 + i % 2],
       CASE WHEN g.r_meta < 0.166 THEN '{}'::jsonb
       ELSE jsonb_strip_nulls(jsonb_build_object(
           'k01', 'C' || substr(md5(i::text), 1, 25),
           'k02', lpad(i::text, 11, '0'),
           'k03', 'Name ' || lpad((i % (:rows / 2))::text, 10, '0'),
           'k04', CASE WHEN i % 100 < 82 THEN 'Location ' || lpad((i % (:rows / 3))::text, 10, '0') END,
           'k05', CASE WHEN i % 100 < 82 THEN 'Site ' || lpad((i % (:rows / 4))::text, 13, '0') END,
           'k06', CASE WHEN i % 100 < 36 THEN lpad((i * 3)::text, 11, '0') END,
           'k07', CASE WHEN i % 5 = 0 THEN lpad((i * 7)::text, 20, '0') END,
           'k08', lpad((i % (:rows / 2))::text, 7, '0'),
           'k09', (i % 99991)::text,
           'k10', lpad((i % (:rows / 3))::text, 7, '0'),
           'k11', (i % 99989)::text,
           'k12', CASE WHEN i % 100 < 82 THEN 'A long company name ' || lpad((i % 2130)::text, 9, '0') END,
           'k13', 'Property ' || lpad((i % 40000)::text, 12, '0'),
           'k14', 'Person ' || lpad((i % (:rows / 2))::text, 14, '0'),
           'k15', 'Unit ' || (i % 900),
           'k16', (ARRAY['a', 'b', 'c', 'd'])[1 + i % 4],
           'k17', (date '2024-01-01' + (i % 700))::text,
           'k18', CASE WHEN i % 4 = 1 THEN lpad((i % 1000000)::text, 10, '0') END,
           'k19', CASE WHEN t.ord > 2 THEN gen_random_uuid()::text END,
           'k20', CASE WHEN t.ord > 2 THEN md5(i::text) || '-' || substr(md5((i * 3)::text), 1, 20) END,
           'k21', CASE WHEN t.ord > 2 THEN 'cid_' || substr(md5((i % 5000)::text), 1, 23) END
       )) END,
       (ARRAY['pending', 'posted', 'archived'])[1 + i % 3],
       (i % 100000)::numeric / 100,
       now() - (i || ' seconds')::interval,
       CASE WHEN i % 50 = 0 THEN now() END,
       i % 7,
       now() - (i || ' seconds')::interval,
       md5((i % 20000)::text)::uuid,
       i % 11,
       CASE WHEN i % 40 = 0 THEN gen_random_uuid() END,
       md5((i % 300)::text)::uuid,
       md5((i / 2)::text)::uuid,
       i % 5,
       i % 10 <> 0,
       t.id,
       (i % 977)::numeric,
       (i % 577)::numeric,
       (i % 377)::numeric,
       (i % 177)::numeric,
       i % 3 = 0,
       CASE WHEN i % 60 = 0 THEN gen_random_uuid() END,
       CASE WHEN i % 60 = 1 THEN gen_random_uuid() END,
       now()
FROM (SELECT i, random() AS r_tenant, random() AS r_meta FROM generate_series(1, :rows) i) g
JOIN tenants t ON g.r_tenant >= t.lo AND g.r_tenant < t.hi;

ANALYZE entries;
SELECT count(*) AS rows, round(avg(length(metadata::text))) AS avg_metadata_len,
       pg_size_pretty(pg_table_size('entries')) AS table_size
FROM entries;

SET maintenance_work_mem = :'mwm';
SET max_parallel_maintenance_workers = :workers;
-- The server log then has the partition boundaries and one line per flush and per merge.
SET log_min_messages = debug1;
\timing on

CREATE INDEX CONCURRENTLY entries_idx ON entries
USING bm25 (id, (side::pdb.literal), (metadata::pdb.literal_normalized),
            (metadata::pdb.simple('alias=metadata_words')), (state::pdb.literal),
            amount, created_at, removed_at, removed_at_version, effective_at, account_id,
            account_version, settlement_id, book_id, transaction_id, transaction_version, live,
            tenant_id, pending_in, pending_out, posted_in, posted_out, synchronous,
            reversed_by_id, reverses_id, updated_at)
WITH (:index_options);

\timing off
RESET log_min_messages;
SELECT segno, num_docs, pg_size_pretty(byte_size) AS size, pg_size_pretty(columnar_bytes) AS columnar
FROM paradedb.index_info('entries_idx') ORDER BY num_docs DESC;

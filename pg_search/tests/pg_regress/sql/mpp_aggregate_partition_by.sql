-- =====================================================================
-- AggregateScan must preserve index partition_by so range-based MPP
-- co-partitioning can apply to aggregate-over-join queries.
--
-- Dataset and GUCs mirror mpp_joinscan / mpp_aggregate, but indexes set
-- partition_by on the equi-join keys. JoinScan already uses that metadata
-- (see mpp_joinscan); AggregateScan must do the same.
--
-- Correctness: serial and MPP result rows must match.
-- Plan: VERBOSE EXPLAIN on the MPP GROUP BY must show
-- HashJoinExec mode=Partitioned with partition=id[…] / partition=file_id[…]
-- on the PgSearchScans (not CollectLeft + broadcast).
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO on;

SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
-- Force parallel even on this tiny dataset; otherwise the cost-based
-- planner picks the serial AggregateScan and MPP never activates.
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

-- =====================================================================
-- Test data (mirrors mpp_aggregate / mpp_joinscan, with partition_by)
-- =====================================================================

CREATE TABLE mpp_agg_pb_files (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);
CREATE TABLE mpp_agg_pb_pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER,
    page_text TEXT,
    size_bytes INTEGER
);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_agg_pb_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(1, 100) AS g;

INSERT INTO mpp_agg_pb_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(101, 200) AS g;

INSERT INTO mpp_agg_pb_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(1, 500) AS g;

INSERT INTO mpp_agg_pb_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(501, 1000) AS g;

RESET paradedb.global_mutable_segment_rows;

ANALYZE mpp_agg_pb_files;
ANALYZE mpp_agg_pb_pages;

-- A serial build keeps the worker-count warning deterministic.
SET max_parallel_maintenance_workers TO 0;

CREATE INDEX mpp_agg_pb_files_idx ON mpp_agg_pb_files
USING bm25 (id, title, content)
WITH (
    key_field='id',
    target_segment_count=3,
    partition_by='id',
    text_fields='{"title": {"fast": true}, "content": {}}'
);
CREATE INDEX mpp_agg_pb_pages_idx ON mpp_agg_pb_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    key_field='id',
    target_segment_count=3,
    partition_by='file_id',
    numeric_fields='{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields='{"page_text": {}}'
);

-- =====================================================================
-- Pass 1: serial baseline (max_parallel_workers_per_gather = 0)
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

SELECT COUNT(*)
FROM mpp_agg_pb_files f JOIN mpp_agg_pb_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section';

SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_agg_pb_files f JOIN mpp_agg_pb_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

-- =====================================================================
-- Pass 2: MPP path — same results, plus VERBOSE EXPLAIN.
--
-- With partition_by preserved, the join under AggregateScan should be
-- HashJoinExec mode=Partitioned with range-partitioned PgSearchScans
-- (partition=id[…] / partition=file_id[…]), not CollectLeft + broadcast.
-- =====================================================================

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_agg_pb_files f JOIN mpp_agg_pb_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

SELECT COUNT(*)
FROM mpp_agg_pb_files f JOIN mpp_agg_pb_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section';

SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_agg_pb_files f JOIN mpp_agg_pb_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE mpp_agg_pb_pages;
DROP TABLE mpp_agg_pb_files;

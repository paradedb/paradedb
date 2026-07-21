-- =====================================================================
-- End-to-end MPP exercise on AggregateScan.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

-- Use the closed chain's default worker count so PG sees enough workers
-- to actually parallelize (with parallel_workers = 1, PG falls into the
-- `Single Copy: true` path and never exercises the MPP shuffle).
SET paradedb.mpp_worker_count TO 4;
SET max_parallel_workers_per_gather TO 4;
SET max_parallel_workers TO 8;
-- Force parallel even on this tiny dataset; otherwise the cost-based
-- planner picks the serial AggregateScan and MPP never activates.
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

-- =====================================================================
-- Test data
-- =====================================================================

CREATE TABLE mpp_files (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);
CREATE TABLE mpp_pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER,
    page_text TEXT,
    size_bytes INTEGER
);

INSERT INTO mpp_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(1, 200) AS g;

INSERT INTO mpp_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(1, 1000) AS g;

CREATE INDEX mpp_files_idx ON mpp_files
USING bm25 (id, title, content)
WITH (
    key_field='id',
    text_fields='{"title": {"fast": true}, "content": {}}'
);

CREATE INDEX mpp_pages_idx ON mpp_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    key_field='id',
    numeric_fields='{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields='{"page_text": {}}'
);

ANALYZE mpp_files;
ANALYZE mpp_pages;

-- =====================================================================
-- Pass 1: serial baseline (max_parallel_workers_per_gather = 0)
--
-- Scalar COUNT(*) without GROUP BY: PG's planner doesn't parallelize
-- scalar aggregates on small datasets (no natural Partial+Final split
-- available), so this only exercises the serial customscan path. The
-- result is the correctness baseline for pass 2.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section';

SELECT COUNT(*)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section';

-- GROUP BY: PG can pick a parallel-aggregate plan for this shape.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

-- =====================================================================
-- Pass 2: MPP path (max_parallel_workers_per_gather = 4). Same queries, same expected results.
--
-- The scalar COUNT(*) case still falls back to serial — PG won't
-- parallelize scalar aggregates at this scale even with the parallel-
-- cost knobs zeroed. The GROUP BY case should flip into a `Gather →
-- Parallel Custom Scan` shape with `Workers Planned > 0`, exercising
-- the MPP DSM init / shm_mq mesh / `NetworkShuffleExec` path.
-- =====================================================================

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section';

SELECT COUNT(*)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE mpp_pages;
DROP TABLE mpp_files;

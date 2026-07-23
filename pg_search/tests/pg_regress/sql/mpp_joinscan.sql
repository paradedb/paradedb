-- =====================================================================
-- End-to-end MPP exercise on JoinScan.
--
-- Same dataset shape as mpp_aggregate.sql but the queries don't
-- aggregate — they project columns through a JOIN under a LIMIT,
-- which is what JoinScan activates on. Two passes: serial baseline
-- (max_parallel_workers_per_gather = 0) and MPP path (max_parallel_workers_per_gather = 4). Results must
-- match across the two passes; the EXPLAIN trees differ.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

SET paradedb.mpp_worker_count TO 4;
SET max_parallel_workers_per_gather TO 4;
SET max_parallel_workers TO 8;
-- Force parallel even on this tiny dataset; otherwise the cost-based
-- planner picks the serial JoinScan and MPP never activates.
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

-- =====================================================================
-- Test data (mirrors mpp_aggregate.sql)
-- =====================================================================

CREATE TABLE mpp_join_files (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);
CREATE TABLE mpp_join_pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER,
    page_text TEXT,
    size_bytes INTEGER
);

CREATE INDEX mpp_join_files_idx ON mpp_join_files
USING bm25 (id, title, content)
WITH (
    key_field='id',
    text_fields='{"title": {"fast": true}, "content": {}}'
);

CREATE INDEX mpp_join_pages_idx ON mpp_join_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    key_field='id',
    numeric_fields='{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields='{"page_text": {}}'
);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_join_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(1, 100) AS g;

INSERT INTO mpp_join_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(101, 200) AS g;

INSERT INTO mpp_join_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(1, 500) AS g;

INSERT INTO mpp_join_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(501, 1000) AS g;

RESET paradedb.global_mutable_segment_rows;

ANALYZE mpp_join_files;
ANALYZE mpp_join_pages;

-- =====================================================================
-- Pass 1: serial baseline (max_parallel_workers_per_gather = 0)
--
-- The non-MPP JoinScan path produces the correctness baseline for
-- pass 2.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.title, p.size_bytes
LIMIT 10;

SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.title, p.size_bytes
LIMIT 10;

-- =====================================================================
-- Pass 2: MPP path (max_parallel_workers_per_gather = 4). Same query, same results.
-- EXPLAIN tree should switch to a `Gather -> Parallel Custom Scan`
-- shape, exercising the JoinScan MPP wiring (DSM init, shm_mq mesh,
-- fragment dispatch, leader-side NetworkCoalesceExec gather).
-- =====================================================================

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.title, p.size_bytes
LIMIT 10;

SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.title, p.size_bytes
LIMIT 10;

-- =====================================================================
-- Pass 3: worker metrics reach the leader's EXPLAIN ANALYZE display.
-- Asserts presence, not content: how many workers launch and how they
-- split the rows isn't pinned, so per-fragment metrics vary run to run.
-- A fragment's row counts appear only once its TaskMetrics crossed the mesh.
-- =====================================================================

CREATE OR REPLACE FUNCTION mpp_explain_analyze_lines(q text) RETURNS SETOF text AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF) ' || q LOOP
    RETURN NEXT r."QUERY PLAN";
  END LOOP;
END $$ LANGUAGE plpgsql;

SELECT count(*) > 0 AS worker_metrics_shown
FROM mpp_explain_analyze_lines(
  'SELECT f.title, p.size_bytes
   FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
   WHERE f.content @@@ ''Section''
   ORDER BY f.title, p.size_bytes
   LIMIT 10'
) AS line
WHERE line LIKE '%output_rows%';

DROP FUNCTION mpp_explain_analyze_lines(text);

-- =====================================================================
-- Pass 4: MPP with heap filter
--
-- A heap filter (like `length(f.title) > 6`) must be evaluated in the
-- worker. This tests that the expression context is properly provided
-- to the worker.
-- =====================================================================

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
  AND length(f.title) > 6
ORDER BY f.title, p.size_bytes
LIMIT 10;

SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
  AND length(f.title) > 6
ORDER BY f.title, p.size_bytes
LIMIT 10;

-- =====================================================================
-- Pass 5: Serial fallback check
--
-- Ensure the same query returns identical results when executed serially.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
  AND length(f.title) > 6
ORDER BY f.title, p.size_bytes
LIMIT 10;

SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
  AND length(f.title) > 6
ORDER BY f.title, p.size_bytes
LIMIT 10;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE mpp_join_pages;
DROP TABLE mpp_join_files;

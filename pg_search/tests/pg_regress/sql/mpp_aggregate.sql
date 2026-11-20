-- =====================================================================
-- End-to-end MPP exercise on AggregateScan.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

-- Use the closed chain's default worker count so PG sees enough workers
-- to actually parallelize (with parallel_workers = 1, PG falls into the
-- `Single Copy: true` path and never exercises the MPP shuffle).
SET max_parallel_workers_per_gather TO 3;
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

CREATE INDEX mpp_files_idx ON mpp_files
USING paradedb (id, (title::pdb.unicode_words('columnar=true')), content);

CREATE INDEX mpp_pages_idx ON mpp_pages
USING paradedb (id, file_id, page_text, size_bytes);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(1, 100) AS g;

INSERT INTO mpp_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(101, 200) AS g;

INSERT INTO mpp_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(1, 500) AS g;

INSERT INTO mpp_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(501, 1000) AS g;

RESET paradedb.global_mutable_segment_rows;

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
WHERE f.content ||| 'Section';

SELECT COUNT(*)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

-- GROUP BY: PG can pick a parallel-aggregate plan for this shape.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

-- SELECT DISTINCT routes through the same DataFusion aggregate path as a
-- GROUP BY with no aggregates. No LIMIT, so JoinScan (which requires one)
-- stays out and AggregateScan owns the dedup.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT f.title
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

SELECT COUNT(*) FROM (
    SELECT DISTINCT f.title
    FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
    WHERE f.content ||| 'Section'
) t;

-- Values, not just the cardinality: a shuffle that drops or duplicates a group
-- while keeping the count right would slip past a COUNT-only check.
SELECT DISTINCT f.title
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
ORDER BY f.title
LIMIT 5;

-- pdb.agg: scalar terms aggregation over join
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "title", "order": {"_key": "asc"}, "size": 5}}')
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

SELECT pdb.agg('{"terms": {"field": "title", "order": {"_key": "asc"}, "size": 5}}')
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

-- pdb.agg: terms aggregation with GROUP BY
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, pdb.agg('{"terms": {"field": "title"}}')
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

SELECT f.title, pdb.agg('{"terms": {"field": "title"}}')
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

-- =====================================================================
-- Pass 2: MPP path (max_parallel_workers_per_gather = 3). Same queries, same expected results.
--
-- The scalar COUNT(*) case still falls back to serial — PG won't
-- parallelize scalar aggregates at this scale even with the parallel-
-- cost knobs zeroed. The GROUP BY case should flip into a `Gather →
-- Parallel Custom Scan` shape with `Workers Planned > 0`, exercising
-- the MPP DSM init / shm_mq mesh / `NetworkShuffleExec` path.
-- =====================================================================

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

SELECT COUNT(*)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

-- DISTINCT under MPP: a GROUP BY with no aggregates, partitioned by the
-- group key across workers. Same expected count as the serial baseline.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT f.title
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

SELECT COUNT(*) FROM (
    SELECT DISTINCT f.title
    FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
    WHERE f.content ||| 'Section'
) t;

-- Values, not just the cardinality: a shuffle that drops or duplicates a group
-- while keeping the count right would slip past a COUNT-only check.
SELECT DISTINCT f.title
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
ORDER BY f.title
LIMIT 5;

-- pdb.agg: scalar terms aggregation under MPP
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "title", "order": {"_key": "asc"}, "size": 5}}')
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

SELECT pdb.agg('{"terms": {"field": "title", "order": {"_key": "asc"}, "size": 5}}')
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

-- pdb.agg: terms aggregation with GROUP BY under MPP
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, pdb.agg('{"terms": {"field": "title"}}')
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

SELECT f.title, pdb.agg('{"terms": {"field": "title"}}')
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

-- =====================================================================
-- Pass 3: the size gate falls back to serial execution for the aggregate
-- path too, and the results match the serial baseline.
-- =====================================================================

SET paradedb.mpp_min_rows TO 1000000000;

CREATE OR REPLACE FUNCTION mpp_agg_explain_analyze_lines(q text) RETURNS SETOF text AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF) ' || q LOOP
    RETURN NEXT r."QUERY PLAN";
  END LOOP;
END $$ LANGUAGE plpgsql;

SELECT count(*) = 0 AS gated_no_distributed_exec
FROM mpp_agg_explain_analyze_lines(
  $$SELECT f.title, COUNT(*), SUM(p.size_bytes)
    FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
    WHERE f.content ||| 'Section'
    GROUP BY f.title
    ORDER BY f.title
    LIMIT 5$$
) AS line
WHERE line LIKE '%DistributedExec%';

SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

SELECT count(*) = 0 AS gated_no_distributed_exec_pdb_agg
FROM mpp_agg_explain_analyze_lines(
  $$SELECT pdb.agg('{"terms": {"field": "title", "order": {"_key": "asc"}, "size": 5}}')
    FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
    WHERE f.content ||| 'Section'$$
) AS line
WHERE line LIKE '%DistributedExec%';

SELECT pdb.agg('{"terms": {"field": "title", "order": {"_key": "asc"}, "size": 5}}')
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.content ||| 'Section';

DROP FUNCTION mpp_agg_explain_analyze_lines(text);
RESET paradedb.mpp_min_rows;

-- =====================================================================
-- Pass 4: DataFusion AggregateScan with parallel BaseScan InitPlan (#7593)
--
-- An uncorrelated ARRAY(SELECT ...) subquery inside pdb.term_set becomes an
-- InitPlan containing Gather -> Parallel Custom Scan (ParadeDB Base Scan).
-- Evaluating that InitPlan from a DataFusion AggregateScan must not fail
-- with an unset es_query_dsa when InitializeParallelDSM runs.
-- =====================================================================

-- Single-table SELECT DISTINCT (exact #7593 shape).
SET max_parallel_workers_per_gather TO 1;
SET debug_parallel_query TO on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT id
FROM mpp_files
WHERE id @@@ pdb.term_set(ARRAY(
    SELECT id
    FROM mpp_files
    WHERE id @@@ pdb.term_set(ARRAY[1, 2])
))
ORDER BY id;

SELECT DISTINCT id
FROM mpp_files
WHERE id @@@ pdb.term_set(ARRAY(
    SELECT id
    FROM mpp_files
    WHERE id @@@ pdb.term_set(ARRAY[1, 2])
))
ORDER BY id;

-- Control: same query without AggregateScan.
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT DISTINCT id
FROM mpp_files
WHERE id @@@ pdb.term_set(ARRAY(
    SELECT id
    FROM mpp_files
    WHERE id @@@ pdb.term_set(ARRAY[1, 2])
))
ORDER BY id;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Multi-table MPP GROUP BY with a parallel BaseScan InitPlan.
SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.id @@@ pdb.term_set(ARRAY(
    SELECT id
    FROM mpp_files
    WHERE id @@@ pdb.term_set(ARRAY[1, 2])
))
GROUP BY f.title
ORDER BY f.title;

SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.id @@@ pdb.term_set(ARRAY(
    SELECT id
    FROM mpp_files
    WHERE id @@@ pdb.term_set(ARRAY[1, 2])
))
GROUP BY f.title
ORDER BY f.title;

-- Serial DataFusion baseline for the join aggregate with InitPlan:
-- also guards against nested PgTokioRuntime::block_on when the InitPlan
-- runs a ColumnarExecState BaseScan locally.
SET max_parallel_workers_per_gather TO 0;
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.id @@@ pdb.term_set(ARRAY(
    SELECT id
    FROM mpp_files
    WHERE id @@@ pdb.term_set(ARRAY[1, 2])
))
GROUP BY f.title
ORDER BY f.title;

-- Control: same join aggregate without AggregateScan.
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.id @@@ pdb.term_set(ARRAY(
    SELECT id
    FROM mpp_files
    WHERE id @@@ pdb.term_set(ARRAY[1, 2])
))
GROUP BY f.title
ORDER BY f.title;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Parallel BaseScan InitPlan inside an aggregate FILTER clause (verifies
-- FilterExpr::SearchTag stays matched with the pre-solved tagged scan query).
SET max_parallel_workers_per_gather TO 3;
SELECT f.title,
       COUNT(*) AS total,
       COUNT(*) FILTER (WHERE f.id @@@ pdb.term_set(ARRAY(
           SELECT id
           FROM mpp_files
           WHERE id @@@ pdb.term_set(ARRAY[1, 2])
       ))) AS in_subset
FROM mpp_files f JOIN mpp_pages p ON f.id = p.file_id
WHERE f.id @@@ pdb.term_set(ARRAY[1, 2, 3])
GROUP BY f.title
ORDER BY f.title;

-- Correlated LATERAL rescan of a DataFusion AggregateScan with an InitPlan
-- predicate (verifies pristine base queries are re-solved per outer row).
SELECT v.k, r.id
FROM (VALUES (1), (2)) AS v(k)
CROSS JOIN LATERAL (
    SELECT DISTINCT id
    FROM mpp_files
    WHERE id @@@ pdb.term_set(ARRAY(
        SELECT id
        FROM mpp_files
        WHERE id @@@ pdb.term(v.k)
    ))
) r
ORDER BY v.k, r.id;

RESET debug_parallel_query;
SET max_parallel_workers_per_gather TO 3;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE mpp_pages;
DROP TABLE mpp_files;

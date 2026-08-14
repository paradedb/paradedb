-- =====================================================================
-- End-to-end MPP exercise on JoinScan.
--
-- Same dataset shape as mpp_aggregate.sql but the queries don't
-- aggregate — they project columns through a JOIN under a LIMIT,
-- which is what JoinScan activates on. Two passes: serial baseline
-- (max_parallel_workers_per_gather = 0) and MPP path (max_parallel_workers_per_gather = 3). Results must
-- match across the two passes; the EXPLAIN trees differ.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_range_partitioned_join TO on;

SET max_parallel_workers_per_gather TO 3;
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
USING paradedb (id, title, content)
WITH (
    key_field='id',
    partition_by='id',
    text_fields='{"title": {"fast": true}, "content": {}}'
);

CREATE INDEX mpp_join_pages_idx ON mpp_join_pages
USING paradedb (id, file_id, page_text, size_bytes)
WITH (
    key_field='id',
    partition_by='file_id',
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
-- Pass 2: MPP path (max_parallel_workers_per_gather = 3). Same query, same results.
-- The Custom Scan should contain a DataFusion DistributedExec, exercising
-- DSM initialization, fragment dispatch, and leader-side network coalescing.
-- =====================================================================

SET max_parallel_workers_per_gather TO 3;

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
-- Pass 3: worker metrics and dynamic-filter pruning reach the leader's
-- EXPLAIN ANALYZE display. Asserts presence, not exact counts: how many
-- workers launch and how they split the rows isn't pinned, so per-fragment
-- metrics vary run to run. A fragment's row counts appear only once its
-- TaskMetrics crossed the mesh.
-- =====================================================================

CREATE OR REPLACE FUNCTION mpp_explain_analyze_lines(q text) RETURNS SETOF text AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF) ' || q LOOP
    RETURN NEXT r."QUERY PLAN";
  END LOOP;
END $$ LANGUAGE plpgsql;

CREATE TEMP TABLE mpp_explain_analyze_output AS
SELECT line
FROM mpp_explain_analyze_lines(
  'SELECT f.title, p.size_bytes
   FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
   WHERE f.content @@@ ''Section''
     AND f.id <= 20
   ORDER BY f.title, p.size_bytes
   LIMIT 10'
) AS line;

SELECT count(*) > 0 AS worker_metrics_shown
FROM mpp_explain_analyze_output
WHERE line LIKE '%output_rows%';

-- Assigned range variants expose one local partition, satisfying DataFusion's
-- dynamic-filter routing condition. A worker may apply the resulting filter
-- either in the batch pre-filter or by pushing it directly into Tantivy.
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM mpp_explain_analyze_output
    WHERE line LIKE '%DistributedExec%'
  ) OR NOT EXISTS (
    SELECT 1
    FROM mpp_explain_analyze_output
    WHERE line LIKE '%partition=%'
  ) THEN
    RAISE EXCEPTION 'expected an MPP range-partitioned join';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM mpp_explain_analyze_output
    WHERE line ~ 'rows_pruned=\{[^}]*:[1-9][0-9]*'
       OR line LIKE '%dynamic_filter_pushdown=%'
  ) THEN
    RAISE EXCEPTION 'expected an MPP range-join worker to apply a dynamic filter';
  END IF;
END $$;

DROP TABLE mpp_explain_analyze_output;

-- =====================================================================
-- Pass 4: MPP with heap filter
--
-- A heap filter (like `length(f.title) > 6`) must be evaluated in the
-- worker. This tests that the expression context is properly provided
-- to the worker.
-- =====================================================================

SET max_parallel_workers_per_gather TO 3;

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
-- Pass 6: outer joins keep the shuffle path
--
-- The co-partitioned range flip applies to inner joins only. A LEFT JOIN
-- must keep the shuffle-based shape and stay correct under MPP.
-- =====================================================================

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, p.size_bytes
FROM mpp_join_files f LEFT JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.title, p.size_bytes
LIMIT 10;

SELECT f.title, p.size_bytes
FROM mpp_join_files f LEFT JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.title, p.size_bytes
LIMIT 10;

SET max_parallel_workers_per_gather TO 0;

SELECT f.title, p.size_bytes
FROM mpp_join_files f LEFT JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.title, p.size_bytes
LIMIT 10;

-- =====================================================================
-- Pass 7: aggregate-on-join range co-partitioning
--
-- Inner join with aggregate scan must also range co-partition.
-- =====================================================================

SET max_parallel_workers_per_gather TO 3;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;

SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;


-- =====================================================================
-- Pass 8: MPP with a parameterized search predicate (issue #5445)
--
-- length(f.title) > $1, with plan_cache_mode=force_generic_plan, keeps
-- $1 unresolved in SearchQueryInput. Compared against the same query run
-- serially (enable_mpp = off) so MPP-vs-serial divergence surfaces as a
-- diff, not just a missing worker_metrics_shown flag (see #5167).
-- =====================================================================

SET paradedb.enable_mpp TO on;
SET max_parallel_workers_per_gather TO 4;
SET plan_cache_mode = force_generic_plan;
PREPARE mpp_join_heapfilter_param(int) AS
SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
  AND length(f.title) > $1
ORDER BY f.title, p.size_bytes
LIMIT 10;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
EXECUTE mpp_join_heapfilter_param(6);

EXECUTE mpp_join_heapfilter_param(6);

-- Reuse the cached generic plan with a value that changes the result set. This
-- proves execution-time rebaking binds the current value, not the first one.
EXECUTE mpp_join_heapfilter_param(7);

SELECT count(*) > 0 AS worker_metrics_shown
FROM mpp_explain_analyze_lines(
  $$EXECUTE mpp_join_heapfilter_param(7)$$
) AS line
WHERE line LIKE '%output_rows%';

SELECT count(*) > 0 AS distributed_exec_shown
FROM mpp_explain_analyze_lines(
  $$EXECUTE mpp_join_heapfilter_param(7)$$
) AS line
WHERE line LIKE '%DistributedExec%';

-- JoinScan owns MPP execution; PostgreSQL must not add a second Gather path.
SELECT count(*) = 0 AS no_postgres_gather
FROM mpp_explain_analyze_lines(
  $$EXECUTE mpp_join_heapfilter_param(7)$$
) AS line
WHERE line LIKE '%Gather%';

DEALLOCATE mpp_join_heapfilter_param;

-- Same query, run serially, to diff against the MPP result above.
SET paradedb.enable_mpp TO off;
SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
  AND length(f.title) > 6
ORDER BY f.title, p.size_bytes
LIMIT 10;

SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
  AND length(f.title) > 7
ORDER BY f.title, p.size_bytes
LIMIT 10;
SET paradedb.enable_mpp TO on;

SET plan_cache_mode = auto;

-- =====================================================================
-- Pass 9: MPP with InitPlan/Subquery Parameter
--
-- Pulling from a table forces an InitPlan rather than a folded constant.
-- The leader must evaluate and solve this before dispatching to workers.
-- ORDER BY id in the subquery keeps the picked row stable across runs.
-- =====================================================================
SET max_parallel_workers_per_gather TO 4;

SELECT count(*) > 0 AS worker_metrics_shown
FROM mpp_explain_analyze_lines(
  $$SELECT f.title, p.size_bytes
    FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
    WHERE f.content @@@ (SELECT content FROM mpp_join_files ORDER BY id LIMIT 1)
    ORDER BY f.title, p.size_bytes
    LIMIT 10$$
) AS line
WHERE line LIKE '%output_rows%';

SELECT count(*) > 0 AS distributed_exec_shown
FROM mpp_explain_analyze_lines(
  $$SELECT f.title, p.size_bytes
    FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
    WHERE f.content @@@ (SELECT content FROM mpp_join_files ORDER BY id LIMIT 1)
    ORDER BY f.title, p.size_bytes
    LIMIT 10$$
) AS line
WHERE line LIKE '%DistributedExec%';

SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ (SELECT content FROM mpp_join_files ORDER BY id LIMIT 1)
ORDER BY f.title, p.size_bytes
LIMIT 10;

-- Execute the identical InitPlan query serially so the expected output directly
-- checks MPP/serial result equivalence, not only MPP worker activity.
SET paradedb.enable_mpp TO off;
SELECT f.title, p.size_bytes
FROM mpp_join_files f JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ (SELECT content FROM mpp_join_files ORDER BY id LIMIT 1)
ORDER BY f.title, p.size_bytes
LIMIT 10;
SET paradedb.enable_mpp TO on;

-- =====================================================================
-- Pass 10: MPP with two parameterized source queries (review: mithuncy)
--
-- Solving multiple parameterized source queries must preserve all
-- rewritten PostgreSQL expression trees until rebaking is complete.
-- Exercises both sources having a rewritten Param-to-Const tree, so it
-- fails regardless of which source the JoinScan traversal visits first.
-- =====================================================================
SET max_parallel_workers_per_gather TO 4;

SET plan_cache_mode = force_generic_plan;

PREPARE mpp_join_two_source_params(int, int) AS
SELECT f.id AS file_id, p.id AS page_id
FROM mpp_join_files f
JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
  AND length(f.title) > $1
  AND p.page_text @@@ 'Page'
  AND length(p.page_text) > $2
ORDER BY f.id, p.id
LIMIT 5;

EXECUTE mpp_join_two_source_params(6, 6);

-- Change both source-local parameter values under the same generic plan.
EXECUTE mpp_join_two_source_params(7, 21);

SELECT count(*) > 0 AS distributed_exec_shown
FROM mpp_explain_analyze_lines(
  $$EXECUTE mpp_join_two_source_params(7, 21)$$
) AS line
WHERE line LIKE '%DistributedExec%';

DEALLOCATE mpp_join_two_source_params;
SET plan_cache_mode = auto;

-- =====================================================================
-- Pass 11: MPP with a join-level parameterized predicate (review: mithuncy)
--
-- A cross-relation OR lives in join_level_predicates rather than either
-- source's scan_info.query, so the clause-wide has_parameters()/
-- has_postgres_expressions() traversal is required to catch $1 here.
-- =====================================================================
SET max_parallel_workers_per_gather TO 4;

SET plan_cache_mode = force_generic_plan;

PREPARE mpp_join_level_param(text) AS
SELECT f.id AS file_id, p.id AS page_id
FROM mpp_join_files f
JOIN mpp_join_pages p ON f.id = p.file_id
WHERE f.content @@@ $1
   OR p.page_text @@@ 'zzzznotpresent'
ORDER BY f.id, p.id
LIMIT 5;

EXECUTE mpp_join_level_param('1');

-- Rebind the join-level predicate to a value with a different result set.
EXECUTE mpp_join_level_param('101');

SELECT count(*) > 0 AS distributed_exec_shown
FROM mpp_explain_analyze_lines(
  $$EXECUTE mpp_join_level_param('101')$$
) AS line
WHERE line LIKE '%DistributedExec%';

DEALLOCATE mpp_join_level_param;
SET plan_cache_mode = auto;

-- =====================================================================
-- Pass 12: MPP under a correlated re-execution (review: mithuncy)
--
-- A JoinScan with a Param-backed predicate, driven per-row from a LATERAL
-- subquery, must re-solve against each outer row's value rather than
-- reusing whatever the previous row's exec_custom_scan already solved and
-- rebaked (JoinScanState::reset() previously left join_clause untouched on
-- rescan, so the second row silently reused the first row's resolved
-- plan). Expects the first file matching each term, not the same file
-- twice.
-- =====================================================================
SET max_parallel_workers_per_gather TO 4;
SET plan_cache_mode = force_generic_plan;

CREATE TEMP TABLE mpp_join_rescan_terms(q text);
INSERT INTO mpp_join_rescan_terms VALUES ('1'), ('101');

SELECT t.q, sub.title
FROM mpp_join_rescan_terms t,
LATERAL (
    SELECT f.title
    FROM mpp_join_files f
    JOIN mpp_join_pages p ON f.id = p.file_id
    WHERE f.content @@@ t.q
    ORDER BY f.title, p.id
    LIMIT 1
) sub
ORDER BY t.q;

SELECT count(*) > 0 AS distributed_exec_shown
FROM mpp_explain_analyze_lines(
  $$SELECT t.q, sub.title
    FROM mpp_join_rescan_terms t,
    LATERAL (
      SELECT f.title
      FROM mpp_join_files f
      JOIN mpp_join_pages p ON f.id = p.file_id
      WHERE f.content @@@ t.q
      ORDER BY f.title, p.id
      LIMIT 1
    ) sub
    ORDER BY t.q$$
) AS line
WHERE line LIKE '%DistributedExec%';

DROP TABLE mpp_join_rescan_terms;
DROP FUNCTION mpp_explain_analyze_lines(text);
SET plan_cache_mode = auto;

SET paradedb.enable_mpp TO off;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE mpp_join_pages;
DROP TABLE mpp_join_files;

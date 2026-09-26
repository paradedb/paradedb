-- =====================================================================
-- Spill regression test for ParadeDB Aggregate Scan (MPP path).
--
-- 80k distinct groups, forced through RepartitionExec via MPP
-- parallelism (max_parallel_workers_per_gather=3), work_mem=2.5MB.
-- Exercises the writer/reader BufFile cursor tracking under concurrent
-- read+write interleaving on the same spill file, which the serial-path
-- test (spilling_buffile_serial.sql) cannot reach.
--
-- Also checks that the same query still fails cleanly with
-- paradedb.spill_to_disk left off (the default).
--
-- Regarding the spill_to_disk OFF case, the failure can currently surface
-- through several known error paths:
--   1. The normal work_mem error with "raise work_mem" guidance.
--   2. RepartitionExec's SpillPool error when the DiskManager is disabled
--      (#6326).
--   3. An MPP "transport receiver detached" error when the failing worker
--      exits before its TaskError reaches the leader (#6327).
--
-- All three are treated as expected failures. Any other error is reported
-- as unexpected so that unrelated regressions do not silently pass.
-- =====================================================================
\i common/common_setup.sql
SET client_min_messages TO warning;
SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

DROP TABLE IF EXISTS mpp_spill_large_files CASCADE;
DROP TABLE IF EXISTS mpp_spill_large_pages CASCADE;

CREATE TABLE mpp_spill_large_files (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE mpp_spill_large_pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER,
    page_text TEXT,
    size_bytes INTEGER
);

CREATE INDEX mpp_spill_large_files_idx ON mpp_spill_large_files
USING bm25 (id, title, content)
WITH (
    text_fields = '{"title": {"fast": true}, "content": {}}'
);

CREATE INDEX mpp_spill_large_pages_idx ON mpp_spill_large_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    numeric_fields = '{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields = '{"page_text": {}}'
);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_spill_large_files (title, content)
SELECT
    'file-' || g,
    'Section ' || g || ' has content for spilling'
FROM generate_series(1, 80000) AS g;

INSERT INTO mpp_spill_large_pages (file_id, page_text, size_bytes)
SELECT
    g,
    'Page text for page ' || g,
    (g * 17) % 4096
FROM generate_series(1, 80000) AS g;

RESET paradedb.global_mutable_segment_rows;

ANALYZE mpp_spill_large_files;
ANALYZE mpp_spill_large_pages;

SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET work_mem = '2.5MB';

CREATE OR REPLACE FUNCTION explain_analyze_lines(q text)
RETURNS SETOF text AS $$
DECLARE
    r record;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, BUFFERS OFF) ' || q
    LOOP
        RETURN NEXT r."QUERY PLAN";
    END LOOP;
END
$$ LANGUAGE plpgsql;

-- GUC off (default): the same overflow must fail.
-- The known expected failure modes are described in the header above.
CREATE TABLE mpp_spill_guc_off_outcome (msg text);

DO $$
DECLARE
    err_text text;
BEGIN
    PERFORM f.title, COUNT(*), SUM(p.size_bytes)
    FROM mpp_spill_large_files f
    JOIN mpp_spill_large_pages p ON f.id = p.file_id
    WHERE f.content @@@ 'Section'
    GROUP BY f.title;

    INSERT INTO mpp_spill_guc_off_outcome
        VALUES ('unexpected success with spill_to_disk off');
EXCEPTION WHEN OTHERS THEN
    GET STACKED DIAGNOSTICS err_text = MESSAGE_TEXT;

    IF err_text LIKE '%raise work_mem%'
       OR err_text LIKE '%DiskManager is disabled%'
       OR err_text LIKE '%transport receiver detached%'
    THEN
        INSERT INTO mpp_spill_guc_off_outcome
            VALUES ('failed with an expected error');
    ELSE
        INSERT INTO mpp_spill_guc_off_outcome
            VALUES ('unexpected error: ' || err_text);
    END IF;
END$$;

SELECT msg FROM mpp_spill_guc_off_outcome;
DROP TABLE mpp_spill_guc_off_outcome;

-- GUC on: the MPP path must spill at least one operator and complete.
SET paradedb.spill_to_disk TO on;

CREATE TEMP TABLE mpp_spill_explain_output AS
SELECT line
FROM explain_analyze_lines(
    'SELECT f.title, COUNT(*), SUM(p.size_bytes)
     FROM mpp_spill_large_files f
     JOIN mpp_spill_large_pages p ON f.id = p.file_id
     WHERE f.content @@@ ''Section''
     GROUP BY f.title'
) AS line;

SELECT bool_or(
    line ~ 'spill_count=(\d*[1-9]|\{[^}]*\d:\s*\d*[1-9])'
) AS something_spilled
FROM mpp_spill_explain_output;

DROP TABLE mpp_spill_explain_output;

-- Correctness: exactly 80000 groups exist, and every group matches the
-- formula (one page per file, size_bytes = (g * 17) % 4096). id is
-- generated in insertion order and matches g exactly, since the table is
-- freshly created above.
SELECT
    COUNT(*) = 80000 AS all_groups_present,
    COUNT(*) FILTER (
        WHERE cnt <> 1 OR total_size <> ((id * 17) % 4096)
    ) = 0 AS all_groups_correct
FROM (
    SELECT f.id, f.title, COUNT(*) AS cnt, SUM(p.size_bytes) AS total_size
    FROM mpp_spill_large_files f
    JOIN mpp_spill_large_pages p ON f.id = p.file_id
    WHERE f.content @@@ 'Section'
    GROUP BY f.id, f.title
) q;

RESET work_mem;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET paradedb.spill_to_disk;
RESET client_min_messages;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;

DROP FUNCTION explain_analyze_lines(text);
DROP TABLE mpp_spill_large_pages CASCADE;
DROP TABLE mpp_spill_large_files CASCADE;

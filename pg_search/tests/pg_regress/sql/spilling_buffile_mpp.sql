-- =====================================================================
-- Spill regression test for ParadeDB Aggregate Scan (MPP path).
--
-- 200k distinct groups, forced through RepartitionExec via MPP
-- parallelism (max_parallel_workers_per_gather=3), work_mem=5MB.
-- Exercises the writer/reader BufFile cursor tracking under concurrent
-- read+write interleaving on the same spill file, which the serial-path
-- test (spilling_buffile_serial.sql) cannot reach.
--
-- Also checks that the same query still fails cleanly with
-- paradedb.spill_to_disk left off (the default).
-- =====================================================================
\i common/common_setup.sql

CREATE EXTENSION IF NOT EXISTS pg_search;
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
    key_field = 'id',
    text_fields = '{"title": {"fast": true}, "content": {}}'
);

CREATE INDEX mpp_spill_large_pages_idx ON mpp_spill_large_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    key_field = 'id',
    numeric_fields = '{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields = '{"page_text": {}}'
);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_spill_large_files (title, content)
SELECT
    'file-' || g,
    'Section ' || g || ' has content for spilling'
FROM generate_series(1, 200000) AS g;

INSERT INTO mpp_spill_large_pages (file_id, page_text, size_bytes)
SELECT
    g,
    'Page text for page ' || g,
    (g * 17) % 4096
FROM generate_series(1, 200000) AS g;

RESET paradedb.global_mutable_segment_rows;

ANALYZE mpp_spill_large_files;
ANALYZE mpp_spill_large_pages;

SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET work_mem = '5MB';

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

-- GUC off (default): the same overflow must fail cleanly from the disabled
-- disk manager, not spill.
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM mpp_spill_large_files f
JOIN mpp_spill_large_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title;

-- GUC on: the FinalPartitioned aggregate spills and completes.
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
    line LIKE '%AggregateExec%' AND line ~ 'spill_count=\{?0?:?\s*[1-9]'
) AS aggregate_spilled
FROM mpp_spill_explain_output;
DROP TABLE mpp_spill_explain_output;

-- Correctness: exactly 200000 groups exist, and every group matches the
-- formula (one page per file, size_bytes = (g * 17) % 4096). id is
-- generated in insertion order and matches g exactly, since the table is
-- freshly created above.
SELECT
    COUNT(*) = 200000 AS all_groups_present,
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
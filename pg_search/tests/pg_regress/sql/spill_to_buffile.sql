-- =====================================================================
-- Spill regression test for ParadeDB Aggregate Scan (serial path).
--
-- 20k distinct groups, forced to a SERIAL AggregateExec (mode=Single)
-- via max_parallel_workers_per_gather=0, work_mem=1MB,
-- query completes by spilling. Spilling is proven via a boolean check
-- on AggregateExec's spill_count metric, and correctness
-- is checked directly: size_bytes = (g * 17) % 4096 with one page per
-- file, so COUNT/SUM per group are hand-verifiable.
-- =====================================================================
CREATE EXTENSION IF NOT EXISTS pg_search;
SET client_min_messages TO warning;
SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
DROP TABLE IF EXISTS  spill_small_files CASCADE;
DROP TABLE IF EXISTS  spill_small_pages CASCADE;
CREATE TABLE  spill_small_files (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);
CREATE TABLE  spill_small_pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER,
    page_text TEXT,
    size_bytes INTEGER
);
CREATE INDEX  spill_small_files_idx ON  spill_small_files
USING bm25 (id, title, content)
WITH (
    key_field = 'id',
    text_fields = '{"title": {"fast": true}, "content": {}}'
);
CREATE INDEX  spill_small_pages_idx ON  spill_small_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    key_field = 'id',
    numeric_fields = '{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields = '{"page_text": {}}'
);
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO  spill_small_files (title, content)
SELECT
    'file-' || g,
    'Section ' || g || ' has content for spilling'
FROM generate_series(1, 20000) AS g;
INSERT INTO  spill_small_pages (file_id, page_text, size_bytes)
SELECT
    g,
    'Page text for page ' || g,
    (g * 17) % 4096
FROM generate_series(1, 20000) AS g;
RESET paradedb.global_mutable_segment_rows;
ANALYZE  spill_small_files;
ANALYZE  spill_small_pages;
CREATE OR REPLACE FUNCTION  explain_analyze_lines(q text)
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
SET work_mem = '1MB';
SET max_parallel_workers_per_gather TO 0;
SET paradedb.spill_to_disk TO on;

CREATE TEMP TABLE spill_explain_output AS
SELECT line
FROM explain_analyze_lines(
    'SELECT f.title, COUNT(*), SUM(p.size_bytes)
     FROM spill_small_files f
     JOIN spill_small_pages p ON f.id = p.file_id
     WHERE f.content @@@ ''Section''
     GROUP BY f.title
     ORDER BY f.title
     LIMIT 5'
) AS line;

SELECT bool_or(
    line LIKE '%AggregateExec%' AND line ~ 'spill_count=\{?0?:?\s*[1-9]'
) AS aggregate_spilled
FROM spill_explain_output;

DROP TABLE spill_explain_output;

-- Correctness: size_bytes = (g * 17) % 4096, one page per file --
-- e.g. 'file-10' -> g=10 -> (10*17) % 4096 = 170.
SELECT f.title, COUNT(*), SUM(p.size_bytes)
FROM  spill_small_files f
JOIN  spill_small_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title
LIMIT 5;
RESET work_mem;
SET paradedb.spill_to_disk TO off;
DROP FUNCTION  explain_analyze_lines(text);
DROP TABLE  spill_small_pages CASCADE;
DROP TABLE  spill_small_files CASCADE;
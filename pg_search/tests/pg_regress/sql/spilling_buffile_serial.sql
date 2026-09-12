-- =====================================================================
-- Spill regression test for ParadeDB Aggregate Scan (serial path).
--
-- 20k distinct groups, forced to a SERIAL AggregateExec (mode=Single) via
-- max_parallel_workers_per_gather=0, work_mem=1.25MB. Spilling is proven
-- via a boolean check on AggregateExec's spill_count metric, and
-- correctness is checked directly against size_bytes = (g * 17) % 4096,
-- one page per file.
--
-- Also checks that the same query still fails cleanly with
-- paradedb.spill_to_disk left off (the default).
-- =====================================================================
\i common/common_setup.sql

CREATE EXTENSION IF NOT EXISTS pg_search;
SET client_min_messages TO warning;
SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

DROP TABLE IF EXISTS spill_small_files CASCADE;
DROP TABLE IF EXISTS spill_small_pages CASCADE;

CREATE TABLE spill_small_files (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE spill_small_pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER,
    page_text TEXT,
    size_bytes INTEGER
);

CREATE INDEX spill_small_files_idx ON spill_small_files
USING bm25 (id, title, content)
WITH (
    key_field = 'id',
    text_fields = '{"title": {"fast": true}, "content": {}}'
);

CREATE INDEX spill_small_pages_idx ON spill_small_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    key_field = 'id',
    numeric_fields = '{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields = '{"page_text": {}}'
);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO spill_small_files (title, content)
SELECT
    'file-' || g,
    'Section ' || g || ' has content for spilling'
FROM generate_series(1, 20000) AS g;

INSERT INTO spill_small_pages (file_id, page_text, size_bytes)
SELECT
    g,
    'Page text for page ' || g,
    (g * 17) % 4096
FROM generate_series(1, 20000) AS g;

RESET paradedb.global_mutable_segment_rows;

ANALYZE spill_small_files;
ANALYZE spill_small_pages;

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

-- GUC off (default): the overflow surfaces the disabled disk manager's own
-- error ("temporary files are not enabled...") as the outer cause, with
-- this pool's "raise work_mem" message chained underneath it via "caused
-- by" -- not the pool's message alone, since try_grow() still runs and
-- still fails; the disk manager just can't act on it by spilling.
SET work_mem = '1.25MB';
SELECT f.id, f.title, COUNT(*) AS cnt, SUM(p.size_bytes) AS total_size
FROM spill_small_files f
JOIN spill_small_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.id, f.title;

-- GUC on: the aggregate spills and completes.
SET paradedb.spill_to_disk TO on;
CREATE TEMP TABLE spill_explain_output AS
SELECT line
FROM explain_analyze_lines(
    'SELECT f.title, COUNT(*), SUM(p.size_bytes)
     FROM spill_small_files f
     JOIN spill_small_pages p ON f.id = p.file_id
     WHERE f.content @@@ ''Section''
     GROUP BY f.title'
) AS line;
SELECT bool_or(
    line LIKE '%AggregateExec%' AND line ~ 'spill_count=\{?0?:?\s*[1-9]'
) AS aggregate_spilled
FROM spill_explain_output;
DROP TABLE spill_explain_output;

-- Correctness: exactly 20000 groups exist, and every group matches the
-- formula (one page per file, size_bytes = (g * 17) % 4096). id is
-- generated in insertion order and matches g exactly, since the table is
-- freshly created above.
SELECT
    COUNT(*) = 20000 AS all_groups_present,
    COUNT(*) FILTER (
        WHERE cnt <> 1 OR total_size <> ((id * 17) % 4096)
    ) = 0 AS all_groups_correct
FROM (
    SELECT f.id, f.title, COUNT(*) AS cnt, SUM(p.size_bytes) AS total_size
    FROM spill_small_files f
    JOIN spill_small_pages p ON f.id = p.file_id
    WHERE f.content @@@ 'Section'
    GROUP BY f.id, f.title
) q;

RESET work_mem;
RESET paradedb.spill_to_disk;
RESET client_min_messages;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;

DROP FUNCTION explain_analyze_lines(text);
DROP TABLE spill_small_pages CASCADE;
DROP TABLE spill_small_files CASCADE;

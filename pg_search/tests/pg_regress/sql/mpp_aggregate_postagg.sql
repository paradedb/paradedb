-- =====================================================================
-- End-to-end MPP exercise on AggregateScan with the post-aggregate peer
-- mesh shuffle (Track A + Track B) flipped on.
--
-- Runs the same aggregate-on-join group-by query under both
-- `paradedb.enable_mpp_postagg_shuffle = off` (legacy single-boundary
-- gather) and `= on` (two-boundary peer-mesh + gather), then computes
-- aggregate totals over each result. The totals must match byte-exact;
-- if they don't, the post-agg path has a routing/correctness bug.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

SET paradedb.mpp_worker_count TO 4;
SET max_parallel_workers_per_gather TO 4;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

CREATE TABLE mpp_pa_files (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);
CREATE TABLE mpp_pa_pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER,
    page_text TEXT,
    size_bytes INTEGER
);

INSERT INTO mpp_pa_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(1, 200) AS g;

INSERT INTO mpp_pa_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(1, 1000) AS g;

CREATE INDEX mpp_pa_files_idx ON mpp_pa_files
USING bm25 (id, title, content)
WITH (
    key_field='id',
    text_fields='{"title": {"fast": true}, "content": {}}'
);

CREATE INDEX mpp_pa_pages_idx ON mpp_pa_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    key_field='id',
    numeric_fields='{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields='{"page_text": {}}'
);

ANALYZE mpp_pa_files;
ANALYZE mpp_pa_pages;

SET paradedb.enable_mpp TO on;

-- =====================================================================
-- Pass 1: legacy single-boundary path (postagg shuffle = off).
-- Establishes the correctness baseline.
-- =====================================================================

SET paradedb.enable_mpp_postagg_shuffle TO off;

SELECT
    COUNT(*)            AS num_groups,
    SUM(c)              AS total_count,
    SUM(s)              AS total_sum
FROM (
    SELECT f.title, COUNT(*) AS c, SUM(p.size_bytes) AS s
    FROM mpp_pa_files f JOIN mpp_pa_pages p ON f.id = p.file_id
    WHERE f.content @@@ 'Section'
    GROUP BY f.title
) t;

-- =====================================================================
-- Pass 2: post-aggregate peer-mesh shuffle path. Same query, byte-exact
-- expected output.
-- =====================================================================

SET paradedb.enable_mpp_postagg_shuffle TO on;

SELECT
    COUNT(*)            AS num_groups,
    SUM(c)              AS total_count,
    SUM(s)              AS total_sum
FROM (
    SELECT f.title, COUNT(*) AS c, SUM(p.size_bytes) AS s
    FROM mpp_pa_files f JOIN mpp_pa_pages p ON f.id = p.file_id
    WHERE f.content @@@ 'Section'
    GROUP BY f.title
) t;

DROP TABLE mpp_pa_pages;
DROP TABLE mpp_pa_files;

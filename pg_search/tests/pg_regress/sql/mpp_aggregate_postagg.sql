-- =====================================================================
-- MPP AggregateScan: multi-stage natural-shape plan coverage.
--
-- Exercises the transport substrate end-to-end via post-aggregate
-- patterns that produce a three-stage plan
-- (NetworkCoalesceExec -> NetworkShuffleExec -> NetworkBroadcastExec):
--   - GROUP BY one key, multiple aggregates
--   - GROUP BY multiple keys
--   - HAVING + ORDER BY + LIMIT
--   - aggregation over a HashJoin with broadcast build subtree
--
-- Each pass runs the same query in serial mode (max_parallel_workers_per_gather=0) then MPP
-- mode (max_parallel_workers_per_gather=4). The expected.out compares them byte-for-byte, so
-- any MPP-vs-serial divergence shows up as a regression.
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
-- Regress tables are tiny; disable the size gate so MPP engages.
SET paradedb.mpp_min_rows TO 0;
SET paradedb.mpp_worker_count TO 4;
SET max_parallel_workers_per_gather TO 4;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

CREATE TABLE mpp_postagg_files (
    id SERIAL PRIMARY KEY,
    title TEXT,
    category TEXT,
    content TEXT
);
CREATE TABLE mpp_postagg_pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER,
    page_text TEXT,
    size_bytes INTEGER
);

CREATE INDEX mpp_postagg_files_idx ON mpp_postagg_files
USING bm25 (id, title, category, content)
WITH (
    key_field='id',
    text_fields='{"title": {"fast": true}, "category": {"fast": true}, "content": {}}'
);

CREATE INDEX mpp_postagg_pages_idx ON mpp_postagg_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    key_field='id',
    numeric_fields='{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields='{"page_text": {}}'
);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_postagg_files (title, category, content)
SELECT 'file-' || g,
       'cat-' || (g % 5),
       'Section ' || g || ' has content for testing'
FROM generate_series(1, 100) AS g;

INSERT INTO mpp_postagg_files (title, category, content)
SELECT 'file-' || g,
       'cat-' || (g % 5),
       'Section ' || g || ' has content for testing'
FROM generate_series(101, 200) AS g;

INSERT INTO mpp_postagg_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(1, 500) AS g;

INSERT INTO mpp_postagg_pages (file_id, page_text, size_bytes)
SELECT (g % 200) + 1,
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(501, 1000) AS g;

ANALYZE mpp_postagg_files;
ANALYZE mpp_postagg_pages;
RESET paradedb.global_mutable_segment_rows;

-- =====================================================================
-- Scenario 1: GROUP BY one key with multiple aggregates.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

SELECT f.category,
       COUNT(*) AS row_count,
       SUM(p.size_bytes) AS total_bytes,
       MIN(p.size_bytes) AS min_bytes,
       MAX(p.size_bytes) AS max_bytes
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.category
ORDER BY f.category;

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.category,
       COUNT(*) AS row_count,
       SUM(p.size_bytes) AS total_bytes,
       MIN(p.size_bytes) AS min_bytes,
       MAX(p.size_bytes) AS max_bytes
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.category
ORDER BY f.category;

SELECT f.category,
       COUNT(*) AS row_count,
       SUM(p.size_bytes) AS total_bytes,
       MIN(p.size_bytes) AS min_bytes,
       MAX(p.size_bytes) AS max_bytes
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.category
ORDER BY f.category;

-- =====================================================================
-- Scenario 2: GROUP BY multiple keys.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

SELECT f.category, f.title, COUNT(*) AS pages_per_file
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.category, f.title
ORDER BY f.category, f.title
LIMIT 10;

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.category, f.title, COUNT(*) AS pages_per_file
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.category, f.title
ORDER BY f.category, f.title
LIMIT 10;

SELECT f.category, f.title, COUNT(*) AS pages_per_file
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.category, f.title
ORDER BY f.category, f.title
LIMIT 10;

-- =====================================================================
-- Scenario 3: HAVING + ORDER BY + LIMIT — late aggregate filtering.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

SELECT f.category, COUNT(*) AS c, SUM(p.size_bytes) AS s
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.category
HAVING COUNT(*) > 100
ORDER BY s DESC
LIMIT 3;

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.category, COUNT(*) AS c, SUM(p.size_bytes) AS s
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.category
HAVING COUNT(*) > 100
ORDER BY s DESC
LIMIT 3;

SELECT f.category, COUNT(*) AS c, SUM(p.size_bytes) AS s
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
GROUP BY f.category
HAVING COUNT(*) > 100
ORDER BY s DESC
LIMIT 3;

-- =====================================================================
-- Scenario 4: Scalar COUNT(*) — falls back to serial (planner caps the
-- task_count for scalar aggregates), but confirms parallel-on doesn't break
-- the serial fallback path. The EXPLAIN under parallel-on documents
-- that the scalar shape does not produce a multi-stage MPP plan.
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

SELECT COUNT(*)
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section';

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*)
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section';

SELECT COUNT(*)
FROM mpp_postagg_files f JOIN mpp_postagg_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section';

-- =====================================================================
-- Scenario 5: Three-table join (two HashJoins → two NetworkBroadcastExec
-- build subtrees under one NetworkShuffleExec shuffle).
--
-- Exercises the walker's nested-parent context propagation across two
-- distinct `stage_id`s simultaneously, and the dispatcher's per-channel
-- EOF on multiple Broadcast routings + the broadcast short-circuit on
-- task_idx > 0 for both build stages.
-- =====================================================================

CREATE TABLE mpp_postagg_categories (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT
);

CREATE INDEX mpp_postagg_categories_idx ON mpp_postagg_categories
USING bm25 (id, name, description)
WITH (
    key_field='id',
    text_fields='{"name": {"fast": true}, "description": {}}'
);

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO mpp_postagg_categories (name, description)
SELECT 'cat-' || g, 'Category ' || g || ' Section description'
FROM generate_series(0, 2) AS g;

INSERT INTO mpp_postagg_categories (name, description)
SELECT 'cat-' || g, 'Category ' || g || ' Section description'
FROM generate_series(3, 4) AS g;

ANALYZE mpp_postagg_categories;
RESET paradedb.global_mutable_segment_rows;

SET max_parallel_workers_per_gather TO 0;

SELECT c.name, COUNT(*) AS row_count, SUM(p.size_bytes) AS total_bytes
FROM mpp_postagg_files f
JOIN mpp_postagg_pages p ON f.id = p.file_id
JOIN mpp_postagg_categories c ON f.category = c.name
WHERE f.content @@@ 'Section'
GROUP BY c.name
ORDER BY c.name;

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT c.name, COUNT(*) AS row_count, SUM(p.size_bytes) AS total_bytes
FROM mpp_postagg_files f
JOIN mpp_postagg_pages p ON f.id = p.file_id
JOIN mpp_postagg_categories c ON f.category = c.name
WHERE f.content @@@ 'Section'
GROUP BY c.name
ORDER BY c.name;

SELECT c.name, COUNT(*) AS row_count, SUM(p.size_bytes) AS total_bytes
FROM mpp_postagg_files f
JOIN mpp_postagg_pages p ON f.id = p.file_id
JOIN mpp_postagg_categories c ON f.category = c.name
WHERE f.content @@@ 'Section'
GROUP BY c.name
ORDER BY c.name;

-- =====================================================================
-- Cleanup
-- =====================================================================

DROP TABLE mpp_postagg_categories;
DROP TABLE mpp_postagg_pages;
DROP TABLE mpp_postagg_files;

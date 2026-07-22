-- =====================================================================
-- Outer joins (LEFT / RIGHT / FULL) through JoinScan, serial and MPP.
--
-- Data is shaped so both sides have unmatched rows: files 1..50 have
-- no pages, and pages with file_id 201..250 have no file. Each query
-- runs in a serial pass and an MPP pass; results must match.
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
-- Test data
-- =====================================================================

CREATE TABLE outer_files (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);
CREATE TABLE outer_pages (
    id SERIAL PRIMARY KEY,
    file_id INTEGER,
    page_text TEXT,
    size_bytes INTEGER
);

INSERT INTO outer_files (title, content)
SELECT 'file-' || g, 'Section ' || g || ' has content for testing'
FROM generate_series(1, 200) AS g;

-- file_id 51..250: files 1..50 stay unmatched, ids 201..250 dangle.
INSERT INTO outer_pages (file_id, page_text, size_bytes)
SELECT 51 + (g % 200),
       'Page text for page ' || g,
       (g * 17) % 4096
FROM generate_series(1, 1000) AS g;

CREATE INDEX outer_files_idx ON outer_files
USING bm25 (id, title, content)
WITH (
    key_field='id',
    text_fields='{"title": {"fast": true}, "content": {}}'
);

CREATE INDEX outer_pages_idx ON outer_pages
USING bm25 (id, file_id, page_text, size_bytes)
WITH (
    key_field='id',
    numeric_fields='{"file_id": {"fast": true}, "size_bytes": {"fast": true}}',
    text_fields='{"page_text": {"fast": true}}'
);

ANALYZE outer_files;
ANALYZE outer_pages;

-- =====================================================================
-- Pass 1: serial JoinScan (max_parallel_workers_per_gather = 0)
-- =====================================================================

SET max_parallel_workers_per_gather TO 0;

-- LEFT: pages preserved, unmatched pages null-extend the files side.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.file_id, f.title
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.file_id, f.title
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

-- LEFT with the null-extended region on top (dangling file_ids).
SELECT p.id, p.file_id, f.title
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.file_id DESC, p.id
LIMIT 10;

-- LEFT: small side preserved, unmatched files null-extend the pages side.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title, p.id AS page_id
FROM outer_files f LEFT JOIN outer_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.id, p.id
LIMIT 10;

SELECT f.id, f.title, p.id AS page_id
FROM outer_files f LEFT JOIN outer_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.id, p.id
LIMIT 10;

-- RIGHT: mirror of the first LEFT.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.file_id, f.title
FROM outer_files f RIGHT JOIN outer_pages p ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.file_id, f.title
FROM outer_files f RIGHT JOIN outer_pages p ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

-- FULL: both directions null-extend. The @@@ predicate sits in the ON
-- clause; in WHERE it would reject null-extended file rows and reduce
-- the join. Search predicates in outer ON clauses decline JoinScan, so
-- this exercises the fallback, not the custom scan. No EXPLAIN: the
-- fallback plan serializes the index OID into the join filter, which
-- changes on every run.
SELECT f.id, p.id AS page_id, p.file_id
FROM outer_files f FULL JOIN outer_pages p ON f.id = p.file_id AND f.content @@@ 'Section'
ORDER BY f.id NULLS LAST, p.id NULLS LAST
LIMIT 10;

-- FULL with the search predicate in WHERE on the pages side: null-
-- extended page rows can't satisfy it, so PG reduces to LEFT
-- (pages preserved) before the join hook runs.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, p.id AS page_id, p.file_id
FROM outer_files f FULL JOIN outer_pages p ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

SELECT f.id, p.id AS page_id, p.file_id
FROM outer_files f FULL JOIN outer_pages p ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

-- Anti-join spelled as LEFT ... IS NULL: only the null-extended rows.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.file_id
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page' AND f.id IS NULL
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.file_id
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page' AND f.id IS NULL
ORDER BY p.id
LIMIT 10;

-- Non-equi ON condition on an outer join: declines JoinScan (the
-- pipeline would misapply it and change null-extension).
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.file_id, f.title
FROM outer_pages p LEFT JOIN outer_files f
    ON f.id = p.file_id AND p.size_bytes > 2048
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.file_id, f.title
FROM outer_pages p LEFT JOIN outer_files f
    ON f.id = p.file_id AND p.size_bytes > 2048
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

-- =====================================================================
-- Pass 2: MPP (max_parallel_workers_per_gather = 4). Same queries,
-- same results.
-- =====================================================================

SET max_parallel_workers_per_gather TO 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.file_id, f.title
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.file_id, f.title
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.file_id, f.title
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.file_id DESC, p.id
LIMIT 10;

SELECT f.id, f.title, p.id AS page_id
FROM outer_files f LEFT JOIN outer_pages p ON f.id = p.file_id
WHERE f.content @@@ 'Section'
ORDER BY f.id, p.id
LIMIT 10;

SELECT p.id, p.file_id, f.title
FROM outer_files f RIGHT JOIN outer_pages p ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

SELECT f.id, p.id AS page_id, p.file_id
FROM outer_files f FULL JOIN outer_pages p ON f.id = p.file_id
WHERE p.page_text @@@ 'Page'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.file_id
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page' AND f.id IS NULL
ORDER BY p.id
LIMIT 10;

-- MPP aggregate over an outer join (the shape from the issue).
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*)
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page';

SELECT COUNT(*)
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page';

-- COUNT(f.id) references the nullable side, so the join survives PG's
-- join removal and the aggregate runs on the DataFusion backend. The
-- distributed plan broadcasts the files side and partitions the pages.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(f.id)
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page';

SELECT COUNT(f.id)
FROM outer_pages p LEFT JOIN outer_files f ON f.id = p.file_id
WHERE p.page_text @@@ 'Page';

DROP TABLE outer_pages;
DROP TABLE outer_files;

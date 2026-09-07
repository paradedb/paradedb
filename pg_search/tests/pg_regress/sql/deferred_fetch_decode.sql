-- A late-materialized string column is looked up in two steps: a fast-field fetch
-- (doc address to term ordinal, TantivyFetchExec) and a dictionary decode (term
-- ordinal to string, TantivyDecodeExec). By default the scan emits doc addresses and
-- both nodes sit next to each other at the decode point. With
-- paradedb.defer_column_fetch = off the scan resolves the ordinals itself, in doc
-- order, and only the decode node is needed above. Both placements must return the
-- same rows.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO off;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;

DROP TABLE IF EXISTS dfd_files CASCADE;
DROP TABLE IF EXISTS dfd_documents CASCADE;

CREATE TABLE dfd_documents (
    id TEXT PRIMARY KEY,
    category TEXT
);

INSERT INTO dfd_documents (id, category) VALUES
('doc-01', 'PROJECT_ALPHA design review'),
('doc-02', 'BETA_GROUP budget overview'),
('doc-03', 'PROJECT_ALPHA roadmap planning'),
('doc-04', 'GAMMA_DIVISION quarterly report'),
('doc-05', 'PROJECT_ALPHA feedback notes'),
('doc-06', 'BETA_GROUP marketing strategy'),
('doc-07', 'PROJECT_ALPHA milestone tracker'),
('doc-08', 'GAMMA_DIVISION vendor evaluation'),
('doc-09', 'PROJECT_ALPHA resource allocation'),
('doc-10', 'BETA_GROUP incident response');

-- `price` is an unconstrained NUMERIC, which the index stores as bytes, so it takes the
-- bytes branches of the fetch and the decode.
CREATE TABLE dfd_files (
    id SERIAL PRIMARY KEY,
    document_id TEXT,
    title TEXT,
    content TEXT,
    price NUMERIC
);

CREATE INDEX dfd_documents_idx ON dfd_documents USING bm25 (id, category)
WITH (key_field = 'id', text_fields = '{"category": {"fast": true}}');

CREATE INDEX dfd_files_idx ON dfd_files USING bm25 (id, document_id, title, content, price)
WITH (key_field = 'id', text_fields = '{"document_id": {"tokenizer": {"type": "keyword"}, "fast": true}, "title": {"fast": true}, "content": {"fast": true}}', numeric_fields = '{"price": {"fast": true}}');

-- Two insert batches after the index exists give the files index two segments, so a
-- batch above the join mixes rows whose ordinals live in different dictionaries.
SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO dfd_files (document_id, title, content, price)
SELECT
    'doc-' || LPAD(((i - 1) % 10 + 1)::TEXT, 2, '0'),
    CASE WHEN i % 7 = 0 THEN NULL ELSE 'File Title ' || LPAD(i::TEXT, 3, '0') END,
    'file content for item ' || i,
    CASE WHEN i % 11 = 0 THEN NULL ELSE ((i * 37) % 101)::NUMERIC / 4 END
FROM generate_series(1, 50) AS i;

INSERT INTO dfd_files (document_id, title, content, price)
SELECT
    'doc-' || LPAD(((i - 1) % 10 + 1)::TEXT, 2, '0'),
    CASE WHEN i % 7 = 0 THEN NULL ELSE 'File Title ' || LPAD(i::TEXT, 3, '0') END,
    'file content for item ' || i,
    CASE WHEN i % 11 = 0 THEN NULL ELSE ((i * 37) % 101)::NUMERIC / 4 END
FROM generate_series(51, 100) AS i;

RESET paradedb.global_mutable_segment_rows;

-- =============================================================================
-- Default: the fetch is deferred next to the decode
-- =============================================================================

SHOW paradedb.defer_column_fetch;

-- Top K on the deferred column: the SegmentedTopKExec goes under the fetch, so the
-- fetch and the decode only run for the K survivors.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

-- NULL titles sort last and decode as NULL.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'BETA_GROUP'
ORDER BY f.title DESC NULLS FIRST, f.id ASC
LIMIT 4;

SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'BETA_GROUP'
ORDER BY f.title DESC NULLS FIRST, f.id ASC
LIMIT 4;

-- Two deferred columns in one sort: both are fetched and decoded together.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title, f.content
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'GAMMA_DIVISION'
ORDER BY f.title DESC NULLS FIRST, f.content ASC
LIMIT 5;

SELECT f.id, f.title, f.content
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'GAMMA_DIVISION'
ORDER BY f.title DESC NULLS FIRST, f.content ASC
LIMIT 5;

-- Bytes-backed column: NULLs first, then the smallest prices.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.price
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.price ASC NULLS FIRST, f.id ASC
LIMIT 7;

SELECT f.id, f.price
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.price ASC NULLS FIRST, f.id ASC
LIMIT 7;

-- Without the SegmentedTopKExec the fetch and decode stay adjacent under the sort.
SET paradedb.enable_segmented_topk = off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

RESET paradedb.enable_segmented_topk;

-- An aggregate over the join decodes its group key above the visibility filter.
SET paradedb.enable_aggregate_late_materialization = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, COUNT(*)
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'BETA_GROUP'
GROUP BY f.title
ORDER BY f.title NULLS LAST
LIMIT 5;

SELECT f.title, COUNT(*)
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'BETA_GROUP'
GROUP BY f.title
ORDER BY f.title NULLS LAST
LIMIT 5;

RESET paradedb.enable_aggregate_late_materialization;

-- =============================================================================
-- Fetch in the scan: the scan emits term ordinals, only the decode is deferred
-- =============================================================================

SET paradedb.defer_column_fetch = off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'BETA_GROUP'
ORDER BY f.title DESC NULLS FIRST, f.id ASC
LIMIT 4;

SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'BETA_GROUP'
ORDER BY f.title DESC NULLS FIRST, f.id ASC
LIMIT 4;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title, f.content
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'GAMMA_DIVISION'
ORDER BY f.title DESC NULLS FIRST, f.content ASC
LIMIT 5;

SELECT f.id, f.title, f.content
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'GAMMA_DIVISION'
ORDER BY f.title DESC NULLS FIRST, f.content ASC
LIMIT 5;

-- Bytes-backed column: NULLs first, then the smallest prices.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.price
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.price ASC NULLS FIRST, f.id ASC
LIMIT 7;

SELECT f.id, f.price
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.price ASC NULLS FIRST, f.id ASC
LIMIT 7;

SET paradedb.enable_segmented_topk = off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

RESET paradedb.enable_segmented_topk;

SET paradedb.enable_aggregate_late_materialization = on;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.title, COUNT(*)
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'BETA_GROUP'
GROUP BY f.title
ORDER BY f.title NULLS LAST
LIMIT 5;

SELECT f.title, COUNT(*)
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'BETA_GROUP'
GROUP BY f.title
ORDER BY f.title NULLS LAST
LIMIT 5;

RESET paradedb.enable_aggregate_late_materialization;

-- =============================================================================
-- MPP: the fetch placement travels with the dispatched plan
-- =============================================================================

SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

RESET paradedb.defer_column_fetch;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

SELECT f.id, f.title
FROM dfd_documents d JOIN dfd_files f ON f.document_id = d.id
WHERE d.category @@@ 'PROJECT_ALPHA'
ORDER BY f.title ASC, f.id ASC
LIMIT 5;

DROP TABLE dfd_files;
DROP TABLE dfd_documents;

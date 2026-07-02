\i common/common_setup.sql

-- Reproduces the query-side half of https://github.com/paradedb/paradedb/issues/5365
--
-- Query-visible mutable-segment materialization used to fetch heap tuples with SnapshotAny,
-- which can select DEAD / RECENTLY_DEAD versions whose external TOAST chunks have already
-- been reclaimed, raising "missing/unexpected chunk number ... in pg_toast_*" under
-- concurrent VACUUM. It now fetches each ctid with the scan's active MVCC snapshot, so a
-- SELECT only detoasts versions whose TOAST is protected by that snapshot.
--
-- This test pins down the *visibility* behavior of that change: a mutable segment holding a
-- mix of live, updated, and deleted large-toasted rows must return exactly the rows visible
-- to the querying statement.

DROP TABLE IF EXISTS data_docstore_query;
CREATE TABLE data_docstore_query (
    id SERIAL PRIMARY KEY,
    doc_text VARCHAR
);

CREATE INDEX data_docstore_query_idx ON data_docstore_query
USING bm25 (id, doc_text)
WITH (key_field=id, mutable_segment_rows=2, background_layer_sizes='0', layer_sizes='1kb, 100kb, 1mb, 10mb', target_segment_count = 4);

-- Large toasted rows that land in mutable segments.
INSERT INTO data_docstore_query (doc_text)
SELECT repeat('BigData_ ', 200000) FROM generate_series(1, 6);

SELECT mutable FROM paradedb.index_info('data_docstore_query_idx') ORDER BY mutable;

-- All six rows are visible and searchable.
SELECT id FROM data_docstore_query WHERE doc_text ||| 'BigData_' ORDER BY id;

-- Update one row (new toasted version) and delete two others, all within mutable segments.
UPDATE data_docstore_query SET doc_text = repeat('BigData_ ', 200000) WHERE id = 3;
DELETE FROM data_docstore_query WHERE id IN (2, 5);

-- The search must reflect current visibility: ids 2 and 5 gone, 3 still present once.
SELECT id FROM data_docstore_query WHERE doc_text ||| 'BigData_' ORDER BY id;
SELECT count(*) FROM data_docstore_query WHERE doc_text ||| 'BigData_';

-- Reading a deleted toasted tuple's content from a mutable segment must still work
-- (snippet forces detoasting of the matched, visible rows).
SELECT id, substring(doc_text, 1, 8) AS prefix
FROM data_docstore_query
WHERE doc_text ||| 'BigData_'
ORDER BY id;

DROP TABLE data_docstore_query;

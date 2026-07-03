\i common/common_setup.sql

-- Reproduces https://github.com/paradedb/paradedb/issues/5365
--
-- VACUUM's ambulkdelete used to materialize mutable segments, which re-fetches and
-- detoasts heap rows. Concurrent with a VACUUM freeing those same TOAST chunks, that
-- raised spurious "missing/unexpected chunk number ... in pg_toast_*" errors. VACUUM
-- now reads a mutable segment's live ctids from its own add/remove log and never
-- touches the heap, so VACUUM over toasted mutable-segment rows must succeed and still
-- reclaim dead rows.

DROP TABLE IF EXISTS data_docstore_vacuum;
CREATE TABLE data_docstore_vacuum (
    id SERIAL PRIMARY KEY,
    doc_text VARCHAR
);

CREATE INDEX data_docstore_vacuum_idx ON data_docstore_vacuum
USING bm25 (id, doc_text)
WITH (key_field=id, mutable_segment_rows=2, background_layer_sizes='0', layer_sizes='1kb, 100kb, 1mb, 10mb', target_segment_count = 4);

-- Several large toasted rows, landing in mutable segments.
INSERT INTO data_docstore_vacuum (doc_text)
SELECT repeat('BigData_ ', 200000) FROM generate_series(1, 6);

SELECT mutable FROM paradedb.index_info('data_docstore_vacuum_idx') ORDER BY mutable;
SELECT count(*) FROM data_docstore_vacuum WHERE doc_text ||| 'BigData_';

-- Delete and update some rows to leave dead heap tuples (with toasted values) that
-- VACUUM must reclaim from the mutable segment.
DELETE FROM data_docstore_vacuum WHERE id IN (2, 4);
UPDATE data_docstore_vacuum SET doc_text = repeat('BigData_ ', 200000) WHERE id = 6;

-- This is the operation that previously errored on toasted mutable-segment rows.
VACUUM data_docstore_vacuum;

-- Dead rows are gone; the rest are still searchable.
SELECT id FROM data_docstore_vacuum WHERE doc_text ||| 'BigData_' ORDER BY id;
SELECT count(*) FROM data_docstore_vacuum WHERE doc_text ||| 'BigData_';

-- A full VACUUM after further churn should also remain error-free.
DO $$
BEGIN
  FOR i IN 1..10 LOOP
    UPDATE data_docstore_vacuum SET doc_text = repeat('BigData_ ', 200000) WHERE id = 1;
  END LOOP;
END$$;
VACUUM data_docstore_vacuum;
SELECT count(*) FROM data_docstore_vacuum WHERE doc_text ||| 'BigData_';

DROP TABLE data_docstore_vacuum;

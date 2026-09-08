\i common/common_setup.sql

DROP TABLE IF EXISTS index_only_scan;
CREATE TABLE index_only_scan (
    id bigint,
    tenant_id bigint NOT NULL,
    row_id int NOT NULL,
    score double precision,
    active boolean,
    body text,
    PRIMARY KEY (tenant_id, row_id)
);

INSERT INTO index_only_scan VALUES
    (1, 10, 1, 1.5, true, 'needle one'),
    (NULL, 10, 2, NULL, false, 'needle two'),
    (1, 20, 1, 3.5, NULL, 'needle three'),
    (NULL, 20, 2, 9.0, true, 'other');

CREATE INDEX index_only_scan_idx
ON index_only_scan
USING paradedb (id, tenant_id, score, active, body);

VACUUM (FREEZE, ANALYZE) index_only_scan;

CREATE FUNCTION explain_index_only(query text) RETURNS SETOF text LANGUAGE plpgsql AS $$
DECLARE
    line text;
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) ' || query LOOP
        RETURN NEXT regexp_replace(line, '"oid":\d+', '"oid":N');
    END LOOP;
END;
$$;

SET paradedb.enable_custom_scan = off;
RESET enable_indexscan;
SET enable_seqscan = off;
SET enable_bitmapscan = off;

-- A non-first fast field is returnable without a configured key field.
SELECT explain_index_only($$SELECT tenant_id FROM index_only_scan WHERE body @@@ 'needle'$$);

-- Multiple fast fields are populated in index tuple order, including NULL values.
SELECT explain_index_only($$SELECT id, tenant_id, score, active FROM index_only_scan WHERE body @@@ 'needle'$$);
SELECT id, tenant_id, score, active
FROM index_only_scan
WHERE body @@@ 'needle'
ORDER BY tenant_id;

-- A tokenized-only field is not losslessly returnable.
SELECT explain_index_only($$SELECT body FROM index_only_scan WHERE body @@@ 'needle'$$);

-- Residual filters need returnable columns too.
SELECT explain_index_only($$SELECT tenant_id FROM index_only_scan WHERE body @@@ 'needle' AND body LIKE '%two%'$$);
SELECT tenant_id FROM index_only_scan WHERE body @@@ 'needle' AND body LIKE '%two%';

-- CTIDs and whole rows requested by the query still require the heap.
SELECT explain_index_only($$SELECT ctid FROM index_only_scan WHERE body @@@ 'needle'$$);
SELECT explain_index_only($$SELECT index_only_scan FROM index_only_scan WHERE body @@@ 'needle'$$);
SELECT explain_index_only($$SELECT tenant_id FROM index_only_scan WHERE body @@@ 'needle' FOR UPDATE$$);

SET enable_indexonlyscan = off;
SELECT explain_index_only($$SELECT tenant_id FROM index_only_scan WHERE body @@@ 'needle'$$);
RESET enable_indexonlyscan;

-- The fallback condition must not prevent a covering partial index from using an index-only scan.
DROP INDEX index_only_scan_idx;
CREATE INDEX index_only_scan_idx ON index_only_scan
USING paradedb (id, tenant_id, score, active, body) WHERE active;
VACUUM (FREEZE, ANALYZE) index_only_scan;
SELECT explain_index_only($$SELECT tenant_id FROM index_only_scan WHERE active AND body @@@ 'needle'$$);
SELECT tenant_id FROM index_only_scan WHERE active AND body @@@ 'needle' ORDER BY tenant_id;

-- A NOT NULL anchor selects the strict helper without requiring the CTID or whole row.
DROP INDEX index_only_scan_idx;
CREATE INDEX index_only_scan_idx ON index_only_scan
USING paradedb (tenant_id, id, score, active, body) WHERE active;
VACUUM (FREEZE, ANALYZE) index_only_scan;
SELECT explain_index_only($$SELECT tenant_id, score FROM index_only_scan WHERE active AND body @@@ 'needle'$$);
SELECT tenant_id, score FROM index_only_scan WHERE active AND body @@@ 'needle' ORDER BY tenant_id;

-- Deleted mutable-segment rows can have missing fast values before PostgreSQL checks visibility.
CREATE TABLE index_only_uuid (id bigint, uuid uuid, body text, age integer)
WITH (autovacuum_enabled = false);
CREATE INDEX index_only_uuid_idx ON index_only_uuid
USING paradedb (id, uuid, (body::pdb.simple), age);
INSERT INTO index_only_uuid VALUES
    (1, '550e8400-e29b-41d4-a716-446655440000', 'needle', 1),
    (2, '550e8400-e29b-41d4-a716-446655440001', 'needle', 2);
VACUUM (FREEZE, ANALYZE) index_only_uuid;
DELETE FROM index_only_uuid WHERE id = 2;
SELECT explain_index_only($$SELECT id FROM index_only_uuid WHERE body @@@ pdb.all() AND age = 0$$);
SELECT id FROM index_only_uuid WHERE body @@@ pdb.all() AND age = 0;
SELECT id, uuid FROM index_only_uuid WHERE body @@@ pdb.all() ORDER BY id;

-- A nullable UUID must also be returnable, including on all-visible heap pages.
INSERT INTO index_only_uuid VALUES (3, NULL, 'needle', 3);
SELECT explain_index_only($$SELECT id, uuid FROM index_only_uuid WHERE body @@@ pdb.all()$$);
SELECT id, uuid FROM index_only_uuid WHERE body @@@ pdb.all() ORDER BY id;
VACUUM (FREEZE, ANALYZE) index_only_uuid;
SELECT explain_index_only($$SELECT id, uuid FROM index_only_uuid WHERE body @@@ pdb.all()$$);
SELECT id, uuid FROM index_only_uuid WHERE body @@@ pdb.all() ORDER BY id;
DROP TABLE index_only_uuid;

-- Lossy bitmap pages must recheck the original CTID-aware predicate.
SET client_min_messages = error;
CREATE TABLE index_only_bitmap (id bigint NOT NULL, body text, padding text);
ALTER TABLE index_only_bitmap ALTER COLUMN padding SET STORAGE PLAIN;
INSERT INTO index_only_bitmap
SELECT i, CASE WHEN i % 2 = 0 THEN 'needle' ELSE 'other' END, repeat('x', 512)
FROM generate_series(1, 30000) i;
CREATE INDEX index_only_bitmap_idx ON index_only_bitmap USING paradedb (id, body);
VACUUM (ANALYZE) index_only_bitmap;

SET paradedb.enable_aggregate_custom_scan = off;
SET enable_indexscan = off;
SET enable_bitmapscan = on;
SET work_mem = '64kB';

DO $$
DECLARE
    plan json;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, FORMAT JSON) SELECT count(*) FROM index_only_bitmap WHERE body @@@ ''needle'''
    INTO plan;
    IF (plan #>> '{0,Plan,Plans,0,Node Type}') IS DISTINCT FROM 'Bitmap Heap Scan'
        OR COALESCE((plan #>> '{0,Plan,Plans,0,Lossy Heap Blocks}')::int, 0) = 0
        OR (plan #>> '{0,Plan,Plans,0,Actual Rows}')::numeric IS DISTINCT FROM 15000
    THEN
        RAISE EXCEPTION 'expected a lossy bitmap scan returning 15000 matches: %', plan;
    END IF;
END;
$$;

SELECT count(*) FROM index_only_bitmap WHERE body @@@ 'needle';
RESET client_min_messages;
RESET work_mem;
RESET enable_indexscan;
RESET paradedb.enable_aggregate_custom_scan;
DROP TABLE index_only_bitmap;

RESET enable_bitmapscan;
RESET enable_seqscan;
RESET paradedb.enable_custom_scan;

DROP FUNCTION explain_index_only(text);
DROP TABLE index_only_scan;

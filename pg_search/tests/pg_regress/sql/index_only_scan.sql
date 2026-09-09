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

CREATE TABLE index_only_lifetime (id bigint, uuid uuid, nullable_uuid uuid, body text);
INSERT INTO index_only_lifetime
SELECT i, md5(i::text)::uuid, CASE WHEN i % 2 = 0 THEN md5(i::text)::uuid END, 'needle'
FROM generate_series(1, 20000) i;
CREATE INDEX index_only_lifetime_idx ON index_only_lifetime USING paradedb (id, uuid, nullable_uuid, body);
VACUUM (FREEZE, ANALYZE) index_only_lifetime;

DO $$
DECLARE
    plan json;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, FORMAT JSON) SELECT id, uuid, nullable_uuid FROM index_only_lifetime WHERE body @@@ ''needle'''
    INTO plan;
    IF plan #>> '{0,Plan,Node Type}' IS DISTINCT FROM 'Index Only Scan'
        OR (plan #>> '{0,Plan,Heap Fetches}')::int IS DISTINCT FROM 0
        OR (plan #>> '{0,Plan,Actual Rows}')::numeric IS DISTINCT FROM 20000
    THEN
        RAISE EXCEPTION 'expected an index-only scan without heap fetches: %', plan;
    END IF;
END;
$$;

-- UUID datums must survive caller resets without accumulating across rows.
DO $$
DECLARE
    scan CURSOR FOR SELECT id, uuid, nullable_uuid FROM index_only_lifetime WHERE body @@@ 'needle';
    row_data record;
    rows_seen int := 0;
    warm_bytes bigint;
    current_bytes bigint;
BEGIN
    OPEN scan;
    LOOP
        FETCH scan INTO row_data;
        EXIT WHEN NOT FOUND;
        IF row_data.uuid IS DISTINCT FROM md5(row_data.id::text)::uuid
            OR row_data.nullable_uuid IS DISTINCT FROM
                (CASE WHEN row_data.id % 2 = 0 THEN md5(row_data.id::text)::uuid END)
        THEN
            RAISE EXCEPTION 'incorrect UUID values: %', row_data;
        END IF;
        rows_seen := rows_seen + 1;
        IF rows_seen IN (100, 20000) THEN
            SELECT sum(total_bytes) INTO current_bytes FROM pg_backend_memory_contexts
            WHERE name = 'pg_search index-only tuple';
            IF current_bytes IS NULL THEN
                RAISE EXCEPTION 'missing index-only tuple context';
            END IF;
            IF rows_seen = 100 THEN
                warm_bytes := current_bytes;
            ELSIF current_bytes > warm_bytes + 8192 THEN
                RAISE EXCEPTION 'tuple memory grew from % to % bytes', warm_bytes, current_bytes;
            END IF;
        END IF;
    END LOOP;
    CLOSE scan;
    IF rows_seen <> 20000 THEN
        RAISE EXCEPTION 'expected 20000 rows, got %', rows_seen;
    END IF;
    IF EXISTS (SELECT FROM pg_backend_memory_contexts WHERE name = 'pg_search index-only tuple') THEN
        RAISE EXCEPTION 'tuple context survived closing the exhausted scan';
    END IF;
END;
$$;

-- Closing an unfinished cursor must release its tuple and temporary datums too.
DO $$
DECLARE
    scan CURSOR FOR SELECT uuid FROM index_only_lifetime WHERE body @@@ 'needle';
    value uuid;
BEGIN
    FOR i IN 1..100 LOOP
        OPEN scan;
        FETCH scan INTO value;
        CLOSE scan;
    END LOOP;
    IF EXISTS (SELECT FROM pg_backend_memory_contexts WHERE name = 'pg_search index-only tuple') THEN
        RAISE EXCEPTION 'tuple context survived early scan termination';
    END IF;
END;
$$;

-- Error cleanup deletes child contexts before dropping the Rust scan state.
DO $$
DECLARE
    scan CURSOR FOR SELECT uuid, 1 / (id - id) FROM index_only_lifetime WHERE body @@@ 'needle';
    row_data record;
BEGIN
    BEGIN
        OPEN scan;
        FETCH scan INTO row_data;
        RAISE EXCEPTION 'expected division by zero';
    EXCEPTION WHEN division_by_zero THEN
        NULL;
    END;
    IF EXISTS (SELECT FROM pg_backend_memory_contexts WHERE name = 'pg_search index-only tuple') THEN
        RAISE EXCEPTION 'tuple context survived error cleanup';
    END IF;
END;
$$;

DROP TABLE index_only_lifetime;

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

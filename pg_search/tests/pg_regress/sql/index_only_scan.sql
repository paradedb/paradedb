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

RESET enable_bitmapscan;
RESET enable_seqscan;
RESET paradedb.enable_custom_scan;

DROP FUNCTION explain_index_only(text);
DROP TABLE index_only_scan;

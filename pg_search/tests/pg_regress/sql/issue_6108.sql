\i common/common_setup.sql

-- Sequential-scan fallback must match NUMERIC key_field values using the
-- index storage (Numeric64 I64 / NumericBytes), not try_from_datum's Str.

SET paradedb.planner_warnings = 'off';

CREATE FUNCTION explain_seqscan(query text) RETURNS SETOF text LANGUAGE plpgsql AS $$
DECLARE
    line text;
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) ' || query LOOP
        RETURN NEXT regexp_replace(line, '"oid":\d+', '"oid":N');
    END LOOP;
END;
$$;

CREATE TABLE issue_6108_bytes (
    id numeric(30,2) PRIMARY KEY,
    body text
);

INSERT INTO issue_6108_bytes
VALUES (-49999, 'alpha'), (-49990, 'alpha'), (7, 'beta');

CREATE INDEX issue_6108_bytes_idx
ON issue_6108_bytes
USING paradedb (id, body)
WITH (key_field = 'id');

CREATE TABLE issue_6108_i64 (
    id numeric(10,2) PRIMARY KEY,
    body text
);

INSERT INTO issue_6108_i64
VALUES (-49999, 'alpha'), (-49990, 'alpha'), (7, 'beta');

CREATE INDEX issue_6108_i64_idx
ON issue_6108_i64
USING paradedb (id, body)
WITH (key_field = 'id');

SELECT id
FROM issue_6108_bytes
WHERE body @@@ pdb.term('alpha')
ORDER BY id;

SELECT id
FROM issue_6108_i64
WHERE body @@@ pdb.term('alpha')
ORDER BY id;

SET paradedb.enable_custom_scan = off;
SET enable_bitmapscan = off;

SELECT bool_or(line LIKE '%Seq Scan%') AS used_seqscan
FROM explain_seqscan($$SELECT id FROM issue_6108_bytes WHERE body @@@ pdb.term('alpha') ORDER BY id$$) AS line;

SELECT id
FROM issue_6108_bytes
WHERE body @@@ pdb.term('alpha')
ORDER BY id;

SELECT bool_or(line LIKE '%Seq Scan%') AS used_seqscan
FROM explain_seqscan($$SELECT id FROM issue_6108_i64 WHERE body @@@ pdb.term('alpha') ORDER BY id$$) AS line;

SELECT id
FROM issue_6108_i64
WHERE body @@@ pdb.term('alpha')
ORDER BY id;

DROP TABLE issue_6108_bytes CASCADE;
DROP TABLE issue_6108_i64 CASCADE;
DROP FUNCTION explain_seqscan(text);

RESET paradedb.enable_custom_scan;
RESET enable_bitmapscan;
RESET paradedb.planner_warnings;

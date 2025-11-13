\i common/common_setup.sql

-- keep background merges alive long enough to verify cancellation
SELECT paradedb.__set_background_merge_delay_ms(5000);
SET client_min_messages = 'WARNING';

DROP TABLE IF EXISTS bgmerge_cancel CASCADE;
CREATE TABLE bgmerge_cancel (
    id   INT,
    data TEXT
);

CREATE INDEX bgmerge_cancel_idx ON bgmerge_cancel USING bm25(id, data)
    WITH (key_field = 'id', background_layer_sizes = '1kb, 2kb');

INSERT INTO bgmerge_cancel
SELECT g, repeat('payload_data', 200)
FROM generate_series(1, 4000) AS g;

DO $$
DECLARE
    attempts     INTEGER := 0;
    merge_count  INTEGER := 0;
BEGIN
    LOOP
        SELECT count(*) INTO merge_count
        FROM paradedb.merge_info('bgmerge_cancel_idx'::regclass);

        EXIT WHEN merge_count > 0 OR attempts >= 400;

        PERFORM pg_sleep(0.05);
        attempts := attempts + 1;
    END LOOP;

    IF merge_count = 0 THEN
        RAISE EXCEPTION 'background merge did not start in time';
    END IF;
END;
$$;

SELECT count(*) > 0 AS merges_before_vacuum
FROM paradedb.merge_info('bgmerge_cancel_idx'::regclass);

VACUUM (INDEX_CLEANUP ON) bgmerge_cancel;

SELECT count(*) = 0 AS merges_after_vacuum
FROM paradedb.merge_info('bgmerge_cancel_idx'::regclass);

SELECT paradedb.__set_background_merge_delay_ms(0);
DROP TABLE bgmerge_cancel;

\i common/common_cleanup.sql

\i common/common_setup.sql

DROP TABLE IF EXISTS paradedb_background_merge_cancel_log;
CREATE TABLE paradedb_background_merge_cancel_log (
    pid         INT,
    cancelled_at TIMESTAMPTZ DEFAULT clock_timestamp()
);

SET paradedb.global_mutable_segment_rows = 0;

-- keep background merges alive long enough to verify cancellation
SELECT paradedb.__set_background_merge_delay_ms(5000);
SELECT paradedb.__set_max_docs_per_segment(25);
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
FROM generate_series(1, 800) AS g;

-- Create dead tuples so VACUUM actually calls ambulkdelete
DELETE FROM bgmerge_cancel WHERE id % 2 = 0;

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

-- Sleep to ensure VACUUM runs while worker is still in sleep cycle
SELECT pg_sleep(1);

VACUUM (INDEX_CLEANUP ON) bgmerge_cancel;

-- Wait (outside VACUUM) for the worker to notice the sentinel and log the cancellation
DO $$
DECLARE
    attempts             INTEGER := 0;
    max_attempts         INTEGER := 100;
    merges_remaining     INTEGER := 0;
    cancellations_logged INTEGER := 0;
BEGIN
    LOOP
        SELECT count(*) INTO merges_remaining
        FROM paradedb.merge_info('bgmerge_cancel_idx'::regclass);

        SELECT count(*) INTO cancellations_logged
        FROM paradedb_background_merge_cancel_log;

        EXIT WHEN (merges_remaining = 0 AND cancellations_logged > 0)
               OR attempts >= max_attempts;

        PERFORM pg_sleep(0.05);
        attempts := attempts + 1;
    END LOOP;

    IF merges_remaining <> 0 THEN
        RAISE EXCEPTION 'background merge still active after waiting % ms',
            attempts * 50;
    END IF;

    IF cancellations_logged = 0 THEN
        RAISE EXCEPTION 'background merge cancellation was not logged';
    END IF;
END;
$$;

SELECT count(*) = 0 AS merges_after_vacuum
FROM paradedb.merge_info('bgmerge_cancel_idx'::regclass);

SELECT count(*) > 0 AS cancellation_logged
FROM paradedb_background_merge_cancel_log;

SELECT paradedb.__set_background_merge_delay_ms(0);
SELECT paradedb.__set_max_docs_per_segment(0);
DROP TABLE bgmerge_cancel;
DROP TABLE paradedb_background_merge_cancel_log;

\i common/common_cleanup.sql

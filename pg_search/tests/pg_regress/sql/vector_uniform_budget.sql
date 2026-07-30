-- One budget, one gate policy: every IVF segment meters the same work-unit
-- budget, and the radius certificate is the only thing that can end a scan
-- early. There is no second budget and no un-metered path to fall into, so
-- this asserts the invariant from the SQL side, over
-- EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON):
--   * every segment reports work_charged > 0 -- it was metered in units;
--   * work_charged <= the segment's cluster count -- the normalization
--     identity's ceiling (an exhaustive scan costs exactly C units), which
--     is what keeps the fraction meaning "this share of the index's work";
--   * the certificate's telemetry fields are present on every segment,
--     which they can only be if the policy ran.
--
-- The negative half of the format contract -- an index built before radii
-- were required, which now fails to open with a REINDEX message -- cannot
-- be built here: this binary writes V2 `.centroids` for every index it
-- creates. That path is covered end-to-end in tantivy
-- (v1_file_errors_with_reindex_hint, which writes a genuine V1 file) and by
-- a two-binary upgrade check recorded in the PR.
--
-- client_min_messages: the IVF merge emits a paradedb::ivf_build timings
-- NOTICE with nondeterministic millisecond values -- keep it out of the
-- captured output, then pin NOTICE for the assertion block's success
-- message (the pgrx server starts at warning).
SET client_min_messages = WARNING;
CREATE EXTENSION IF NOT EXISTS vector;
\i common/common_setup.sql

DROP TABLE IF EXISTS budget_probe;
CREATE TABLE budget_probe (
    id  int PRIMARY KEY,
    vec vector(16)
);

-- Same forcing recipe as vector_merge: immutable inserts, foreground merges,
-- one clustered segment, replicas present so dedup is exercised.
CREATE INDEX budget_probe_idx ON budget_probe
    USING bm25 (id, vec vector_l2_ops)
    WITH (
        key_field = id,
        cluster_replication = 2,
        target_segment_count = 1,
        mutable_segment_rows = 0,
        layer_sizes = '600kb',
        background_layer_sizes = '0'
    );

INSERT INTO budget_probe
SELECT g, ('[' || repeat((g % 89)::text || ',', 15) || (g % 89)::text || ']')::vector
FROM generate_series(1, 15000) g;

-- The merge produced a clustered segment to meter.
SELECT bool_or(vector_format = 'ivf') AS has_ivf
FROM paradedb.vector_info('budget_probe_idx', 'vec');

SET client_min_messages = notice;

DO $$
DECLARE
    plan jsonb;
    scan jsonb;
    seg_info jsonb;
    seg record;
    charged double precision;
    centroids bigint;
    metered int := 0;
BEGIN
    EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON, COSTS OFF, TIMING OFF, SUMMARY OFF)
             SELECT id FROM budget_probe
             WHERE id @@@ paradedb.all()
             ORDER BY vec <-> ' || quote_literal('[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]') || '::vector
             LIMIT 10'
    INTO plan;

    scan := jsonb_path_query_first(plan, '$[0].** ? (exists(@."Segment Info"))');
    IF scan IS NULL THEN
        RAISE EXCEPTION 'no plan node carries Segment Info: %', plan;
    END IF;
    -- add_json emits Segment Info as a TEXT property whose value is JSON.
    seg_info := (scan ->> 'Segment Info')::jsonb;

    FOR seg IN SELECT key AS segno, value AS info FROM jsonb_each(seg_info)
    LOOP
        -- Flat segments have no clusters and no budget; skip them.
        SELECT sum(vector_num_centroids) INTO centroids
        FROM paradedb.vector_info('budget_probe_idx', 'vec')
        WHERE vector_format = 'ivf' AND segno = seg.segno;
        IF centroids IS NULL THEN
            CONTINUE;
        END IF;

        charged := (seg.info ->> 'work_charged')::double precision;
        IF charged IS NULL THEN
            RAISE EXCEPTION 'segment % reports no work_charged: %', seg.segno, seg.info;
        END IF;
        IF charged <= 0 THEN
            RAISE EXCEPTION 'segment % was not metered (work_charged = %)', seg.segno, charged;
        END IF;
        -- Capacity is the cluster count, exactly; nothing may charge past it.
        IF charged > centroids::double precision + 1e-3 THEN
            RAISE EXCEPTION 'segment % charged % units past its capacity of %',
                seg.segno, charged, centroids;
        END IF;
        -- The certificate ran: its telemetry is folded in unconditionally.
        IF NOT (seg.info ? 'radius_skips') OR NOT (seg.info ? 'gate_armed_at_probe') THEN
            RAISE EXCEPTION 'segment % lacks certificate telemetry: %', seg.segno, seg.info;
        END IF;
        metered := metered + 1;
    END LOOP;

    IF metered < 1 THEN
        RAISE EXCEPTION 'no IVF segment was metered: %', seg_info;
    END IF;

    RAISE NOTICE 'work-unit budget metered % IVF segment(s), all within capacity', metered;
END
$$;

RESET client_min_messages;

DROP TABLE budget_probe;

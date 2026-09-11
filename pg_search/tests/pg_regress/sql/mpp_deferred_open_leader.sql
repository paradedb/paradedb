-- A NOT IN subquery on the probe side of a broadcast join: the distributed planner caps the
-- null-aware anti join to one task and leaves it in the leader's stage, so the leader itself
-- must scan the 2-segment jn_excl leaf. Its reader is built from the manifest captured at
-- query start (no second open), replaying the same view the workers open; the query keeps its
-- MPP launch and returns the serial result.
CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;

CREATE TABLE mdo_items (id bigint PRIMARY KEY, txt text NOT NULL);
CREATE TABLE mdo_excl  (id bigint PRIMARY KEY, val bigint);
CREATE TABLE mdo_small (id bigint PRIMARY KEY, val bigint NOT NULL, txt text NOT NULL);
CREATE INDEX mdo_items_idx ON mdo_items USING paradedb (id, (txt::pdb.literal))
  WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0');
CREATE INDEX mdo_excl_idx ON mdo_excl USING paradedb (id, val)
  WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0');
CREATE INDEX mdo_small_idx ON mdo_small USING paradedb (id, val, (txt::pdb.literal))
  WITH (key_field = 'id', target_segment_count = 8, background_layer_sizes = '0');

-- Two committed batches per table: two immutable segments per index.
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mdo_items SELECT s, 'match' FROM generate_series(1, 2000) s;
INSERT INTO mdo_items SELECT s, 'match' FROM generate_series(2001, 4000) s;
INSERT INTO mdo_excl  SELECT s, s FROM generate_series(50, 300) s;
INSERT INTO mdo_excl  SELECT s, s FROM generate_series(301, 600) s;
INSERT INTO mdo_small SELECT s, s, 'small' FROM generate_series(1, 20) s;
INSERT INTO mdo_small SELECT s, s, 'small' FROM generate_series(21, 40) s;
RESET paradedb.global_mutable_segment_rows;
ANALYZE mdo_items; ANALYZE mdo_excl; ANALYZE mdo_small;

SET max_parallel_workers_per_gather = 4;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;

-- The 2-segment mdo_excl leaf must sit in the leader's box: the lines before the first stage
-- separator. Asserted structurally rather than by storing the whole plan, which is sensitive
-- to DataFusion's rendering.
CREATE TEMP TABLE mdo_outcome (line text);
DO $$
DECLARE
    r record;
    in_leader_box boolean := true;
    leaf_in_leader boolean := false;
    leaf_seen boolean := false;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (VERBOSE, COSTS OFF)
        SELECT i.id FROM mdo_items i JOIN mdo_small s ON s.val = i.id
        WHERE i.txt === ''match'' AND s.txt === ''small''
          AND i.id NOT IN (SELECT val FROM mdo_excl WHERE id @@@ pdb.all())
        ORDER BY i.id LIMIT 5'
    LOOP
        IF r."QUERY PLAN" LIKE '%└──%' THEN
            in_leader_box := false;
        END IF;
        IF r."QUERY PLAN" LIKE '%table=mdo_excl, segments=2%' THEN
            leaf_seen := true;
            leaf_in_leader := leaf_in_leader OR in_leader_box;
        END IF;
    END LOOP;
    IF NOT leaf_seen THEN
        INSERT INTO mdo_outcome VALUES ('plan: UNEXPECTED, no 2-segment mdo_excl leaf');
    ELSIF leaf_in_leader THEN
        INSERT INTO mdo_outcome VALUES ('plan: mdo_excl leaf is leader-hosted');
    ELSE
        INSERT INTO mdo_outcome VALUES ('plan: UNEXPECTED, mdo_excl leaf is worker-hosted');
    END IF;
END$$;

-- JoinScan: the leader opens and scans mdo_excl itself.
SELECT i.id FROM mdo_items i JOIN mdo_small s ON s.val = i.id
WHERE i.txt === 'match' AND s.txt === 'small'
  AND i.id NOT IN (SELECT val FROM mdo_excl WHERE id @@@ pdb.all())
ORDER BY i.id LIMIT 5;

-- AggregateScan takes the same shape.
SELECT count(*) FROM (
  SELECT i.id FROM mdo_items i JOIN mdo_small s ON s.val = i.id
  WHERE i.txt === 'match' AND s.txt === 'small'
    AND i.id NOT IN (SELECT val FROM mdo_excl WHERE id @@@ pdb.all())) q;

-- Both still launch workers: opening the leaf in the leader must not cost the MPP run.
DO $$
DECLARE
    r record;
    launched int := -1;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)
        SELECT i.id FROM mdo_items i JOIN mdo_small s ON s.val = i.id
        WHERE i.txt === ''match'' AND s.txt === ''small''
          AND i.id NOT IN (SELECT val FROM mdo_excl WHERE id @@@ pdb.all())
        ORDER BY i.id LIMIT 5'
    LOOP
        IF r."QUERY PLAN" LIKE '%MPP Launch:%' THEN
            launched := (regexp_match(r."QUERY PLAN", 'workers=(\d+)'))[1]::int;
        END IF;
    END LOOP;
    IF launched >= 2 THEN
        INSERT INTO mdo_outcome VALUES ('joinscan: launched workers');
    ELSE
        INSERT INTO mdo_outcome VALUES ('joinscan: UNEXPECTED launched=' || launched);
    END IF;
END$$;
DO $$
DECLARE
    r record;
    launched int := -1;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)
        SELECT count(*) FROM (
          SELECT i.id FROM mdo_items i JOIN mdo_small s ON s.val = i.id
          WHERE i.txt === ''match'' AND s.txt === ''small''
            AND i.id NOT IN (SELECT val FROM mdo_excl WHERE id @@@ pdb.all())) q'
    LOOP
        IF r."QUERY PLAN" LIKE '%MPP Launch:%' THEN
            launched := (regexp_match(r."QUERY PLAN", 'workers=(\d+)'))[1]::int;
        END IF;
    END LOOP;
    IF launched >= 2 THEN
        INSERT INTO mdo_outcome VALUES ('aggregatescan: launched workers');
    ELSE
        INSERT INTO mdo_outcome VALUES ('aggregatescan: UNEXPECTED launched=' || launched);
    END IF;
END$$;
SELECT line FROM mdo_outcome ORDER BY line;

-- Serial parity.
SET paradedb.enable_join_custom_scan = off;
SET paradedb.enable_aggregate_custom_scan = off;
SELECT i.id FROM mdo_items i JOIN mdo_small s ON s.val = i.id
WHERE i.txt === 'match' AND s.txt === 'small'
  AND i.id NOT IN (SELECT val FROM mdo_excl WHERE id @@@ pdb.all())
ORDER BY i.id LIMIT 5;
SELECT count(*) FROM (
  SELECT i.id FROM mdo_items i JOIN mdo_small s ON s.val = i.id
  WHERE i.txt === 'match' AND s.txt === 'small'
    AND i.id NOT IN (SELECT val FROM mdo_excl WHERE id @@@ pdb.all())) q;

DROP TABLE mdo_items, mdo_excl, mdo_small CASCADE;

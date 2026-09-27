-- Issue #6403: a TopK retry re-claims each (consumer, segment) bitmap stream.
-- TopK queries again with a larger chunk when the first batch loses rows to
-- visibility, and every query builds a fresh scorer per segment. The stream
-- claim was take-once, so the second scorer raised "claimed twice". Dead rows
-- the indexes still point at force the retry deterministically.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET client_min_messages = warning;
SET max_parallel_workers_per_gather TO 0;
SET paradedb.enable_bitmap_intersection TO on;

DROP TABLE IF EXISTS memory_units CASCADE;
-- Autovacuum off: the dead rows below are the trigger and must survive to the query.
CREATE TABLE memory_units (
    id bigserial PRIMARY KEY,
    bank_id text NOT NULL,
    fact_type text NOT NULL,
    body text NOT NULL
) WITH (autovacuum_enabled = off);

CREATE INDEX idx_mu_bank_fact ON memory_units (bank_id, fact_type);
-- No mutable segment, so each INSERT below is its own segment: three segments
-- give the parallel plan two workers on every machine.
CREATE INDEX idx_mu_search ON memory_units USING bm25 (id, body)
    WITH (key_field = 'id', mutable_segment_rows = '0');

INSERT INTO memory_units (bank_id, fact_type, body)
SELECT 'bank' || (i % 4),
       CASE WHEN i % 3 = 0 THEN 'observation' ELSE 'other' END,
       'alpha beta gamma delta ' || i
FROM generate_series(1, 7000) i;
INSERT INTO memory_units (bank_id, fact_type, body)
SELECT 'bank' || (i % 4),
       CASE WHEN i % 3 = 0 THEN 'observation' ELSE 'other' END,
       'alpha beta gamma delta ' || i
FROM generate_series(7001, 14000) i;
INSERT INTO memory_units (bank_id, fact_type, body)
SELECT 'bank' || (i % 4),
       CASE WHEN i % 3 = 0 THEN 'observation' ELSE 'other' END,
       'alpha beta gamma delta ' || i
FROM generate_series(14001, 20000) i;
ANALYZE memory_units;

-- bank1 observations are id = 9 mod 12. Keep every 50th and delete the rest
-- without a VACUUM: both indexes still carry the dead ctids, so the first TopK
-- batch is mostly invisible and the exec method has to query again.
DELETE FROM memory_units
WHERE bank_id = 'bank1' AND fact_type = 'observation' AND id % 50 <> 9;
SELECT count(*) AS visible
FROM memory_units WHERE bank_id = 'bank1' AND fact_type = 'observation';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM memory_units
WHERE bank_id = 'bank1' AND fact_type = 'observation'
  AND id @@@ paradedb.boolean(should => ARRAY[paradedb.match('body', 'alpha')])
ORDER BY paradedb.score(id) DESC, id LIMIT 10;

-- Serial: private iterators, so a re-claim simply opens a fresh one.
SELECT id FROM memory_units
WHERE bank_id = 'bank1' AND fact_type = 'observation'
  AND id @@@ paradedb.boolean(should => ARRAY[paradedb.match('body', 'alpha')])
ORDER BY paradedb.score(id) DESC, id LIMIT 10;

-- Parity with the bitmap off.
SET paradedb.enable_bitmap_intersection TO off;
SELECT id FROM memory_units
WHERE bank_id = 'bank1' AND fact_type = 'observation'
  AND id @@@ paradedb.boolean(should => ARRAY[paradedb.match('body', 'alpha')])
ORDER BY paradedb.score(id) DESC, id LIMIT 10;
SET paradedb.enable_bitmap_intersection TO on;

-- Parallel: the leader pre-mints one shared iterator state per pass, and a
-- participant's retry takes the next state for its segment, so every pass probes
-- the same bitmap and the Top K offset stays valid. Two workers plus the leader
-- over three segments, so each retry happens in a different process.
SET max_parallel_workers_per_gather TO 2;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;
SET min_parallel_table_scan_size TO 0;
SET min_parallel_index_scan_size TO 0;
SET paradedb.min_rows_per_worker TO 0;
SET debug_parallel_query TO on;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM memory_units
WHERE bank_id = 'bank1' AND fact_type = 'observation'
  AND id @@@ paradedb.boolean(should => ARRAY[paradedb.match('body', 'alpha')])
ORDER BY paradedb.score(id) DESC, id LIMIT 10;
SELECT id FROM memory_units
WHERE bank_id = 'bank1' AND fact_type = 'observation'
  AND id @@@ paradedb.boolean(should => ARRAY[paradedb.match('body', 'alpha')])
ORDER BY paradedb.score(id) DESC, id LIMIT 10;
RESET debug_parallel_query;
RESET paradedb.min_rows_per_worker;
RESET min_parallel_index_scan_size;
RESET min_parallel_table_scan_size;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
SET max_parallel_workers_per_gather TO 0;

-- Unordered TopK with a window aggregate runs a second, standalone search on
-- the same source after the first one.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER ()
FROM memory_units
WHERE bank_id = 'bank1' AND fact_type = 'observation'
  AND id @@@ paradedb.boolean(should => ARRAY[paradedb.match('body', 'alpha')])
LIMIT 10;

SELECT id, pdb.agg('{"value_count": {"field": "id"}}'::jsonb) OVER ()
FROM memory_units
WHERE bank_id = 'bank1' AND fact_type = 'observation'
  AND id @@@ paradedb.boolean(should => ARRAY[paradedb.match('body', 'alpha')])
LIMIT 10;

DROP TABLE memory_units;
RESET max_parallel_workers_per_gather;
RESET paradedb.enable_bitmap_intersection;

-- Regression test for #6404: parallel Base Scan workers must report their query
-- telemetry in EXPLAIN ANALYZE. The bug (from #6348) took the parallel handle in
-- shutdown_custom_scan for every process, so workers published nothing and both the
-- per-worker `query_count` and the aggregate `Queries` came back 0.
--
-- The exact EXPLAIN text is non-deterministic (worker ids, segment claims), so we
-- assert only the invariants: workers were launched, the reported `Queries` count is
-- greater than 0, and every worker that claimed a segment reports a non-zero
-- `query_count`. Workers add to their count in the DSM when they run a query, so this
-- holds even when the leader stops at its LIMIT before a worker's shutdown hook runs.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 2;
SET parallel_leader_participation = off;
SET enable_indexscan TO off;

-- Parse "Workers Launched: N", the aggregate "Queries: N" and the per-worker
-- "Parallel Workers: {...}" JSON out of EXPLAIN ANALYZE VERBOSE for `query`.
CREATE FUNCTION worker_telemetry(
    query text,
    OUT workers_launched_gt_0 boolean,
    OUT queries_gt_0 boolean,
    OUT every_claimer_counted boolean
) LANGUAGE plpgsql AS $$
DECLARE
    line     text;
    launched int := 0;
    queries  int := 0;
    workers  jsonb := '{}';
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF) ' || query
    LOOP
        IF line ~ 'Workers Launched:' THEN
            launched := substring(line from 'Workers Launched:\s*([0-9]+)')::int;
        END IF;
        IF line ~ 'Queries:' THEN
            queries := substring(line from 'Queries:\s*([0-9]+)')::int;
        END IF;
        IF line ~ 'Parallel Workers:' THEN
            workers := substring(line from 'Parallel Workers:\s*(.*)$')::jsonb;
        END IF;
    END LOOP;
    workers_launched_gt_0 := launched > 0;
    queries_gt_0 := queries > 0;
    every_claimer_counted := workers <> '{}' AND NOT EXISTS (
        SELECT FROM jsonb_each(workers) w
        WHERE jsonb_array_length(w.value -> 'claimed_segments') > 0
          AND (w.value ->> 'query_count')::int = 0
    );
END $$;

DROP TABLE IF EXISTS tel_probe;
CREATE TABLE tel_probe (id SERIAL8 PRIMARY KEY, uuid UUID, age INTEGER, rating INTEGER);
CREATE INDEX tel_probe_idx ON tel_probe USING paradedb (id, uuid, age)
WITH (
    text_fields = '{"uuid": {"tokenizer": {"type": "keyword"}, "fast": true}}',
    numeric_fields = '{"age": {"fast": true}}'
);

-- One segment per insert, so multiple workers can each claim work.
SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO tel_probe (uuid, age, rating)
SELECT rpad(lpad((i * 7919)::text, 10, '0'), 32, '0')::uuid, 20, 4 FROM generate_series(1, 6) i;
INSERT INTO tel_probe (uuid, age, rating)
SELECT rpad(lpad((i * 7919)::text, 10, '0'), 32, '0')::uuid, 20, 4 FROM generate_series(7, 12) i;
INSERT INTO tel_probe (uuid, age, rating)
SELECT rpad(lpad((i * 7919)::text, 10, '0'), 32, '0')::uuid, 20, 4 FROM generate_series(13, 18) i;
RESET paradedb.global_mutable_segment_rows;
ANALYZE tel_probe;

-- The LIMIT is larger than the table, so the leader reads every row.
SELECT * FROM worker_telemetry(
    'SELECT age FROM tel_probe WHERE rating = 4 AND age @@@ ''20'' ORDER BY uuid LIMIT 100'
);

DROP TABLE tel_probe;

-- The leader stops at its LIMIT while a worker that ran its query is still blocked on
-- its full tuple queue (1 kB of `pad` per row), so its shutdown hook has not run yet.
CREATE TABLE early_probe (id SERIAL8 PRIMARY KEY, ord INTEGER, grp INTEGER, rating INTEGER, pad TEXT);
CREATE INDEX early_probe_idx ON early_probe USING paradedb (id, ord, grp)
WITH (numeric_fields = '{"ord": {"fast": true}, "grp": {"fast": true}}');

SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO early_probe (ord, grp, rating, pad)
SELECT i, 1, 4, (SELECT string_agg(md5(random()::text || g::text), '') FROM generate_series(1, 32) g WHERE i > 0)
FROM generate_series(1, 400) i;
INSERT INTO early_probe (ord, grp, rating, pad)
SELECT 1000 + i, 1, 4, (SELECT string_agg(md5(random()::text || g::text), '') FROM generate_series(1, 32) g WHERE i > 0)
FROM generate_series(1, 400) i;
INSERT INTO early_probe (ord, grp, rating, pad)
SELECT 2000 + i, 1, 4, (SELECT string_agg(md5(random()::text || g::text), '') FROM generate_series(1, 32) g WHERE i > 0)
FROM generate_series(1, 400) i;
RESET paradedb.global_mutable_segment_rows;
VACUUM ANALYZE early_probe;

SELECT * FROM worker_telemetry(
    'SELECT pad FROM early_probe WHERE rating = 4 AND grp @@@ ''1'' ORDER BY ord LIMIT 300'
);

DROP TABLE early_probe;
DROP FUNCTION worker_telemetry(text);

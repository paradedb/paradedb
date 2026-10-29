-- An InitPlan can consume the only worker before the aggregate launches.
CREATE TABLE aggregate_worker_shortage (id int, body text, hash text);
INSERT INTO aggregate_worker_shortage
SELECT i, 'test content ' || i, md5(i::text) FROM generate_series(1, 1000) i;
CREATE INDEX aggregate_worker_shortage_idx
ON aggregate_worker_shortage USING paradedb (id, body, hash);
SET max_parallel_workers = 1;
SET max_parallel_workers_per_gather = 2;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET min_parallel_table_scan_size = 0;
SET parallel_leader_participation = off;
DO $$
DECLARE
    plan jsonb;
BEGIN
    EXECUTE 'EXPLAIN (FORMAT JSON) SELECT count(*) FROM aggregate_worker_shortage
             WHERE body @@@ paradedb.all()
               AND hash = ANY(ARRAY(SELECT hash FROM aggregate_worker_shortage LIMIT 5))'
    INTO plan;
    ASSERT jsonb_path_exists(plan, '$.**."Workers Planned" ? (@ > 0)');
    ASSERT plan::text LIKE '%ParadeDB Aggregate Scan%';
    ASSERT plan::text LIKE '%InitPlan%';
END
$$;
SELECT count(*) FROM aggregate_worker_shortage
WHERE body @@@ paradedb.all()
  AND hash = ANY(ARRAY(SELECT hash FROM aggregate_worker_shortage LIMIT 5));
SELECT count(*) FROM aggregate_worker_shortage
WHERE body @@@ paradedb.all()
  AND hash = ANY(ARRAY(SELECT hash FROM aggregate_worker_shortage LIMIT 5));
SELECT count(*) FROM aggregate_worker_shortage
WHERE body @@@ paradedb.all()
  AND hash = ANY(ARRAY(SELECT hash FROM aggregate_worker_shortage LIMIT 5));
SELECT count(*) FROM aggregate_worker_shortage
WHERE body @@@ paradedb.all()
  AND hash = ANY(ARRAY(SELECT hash FROM aggregate_worker_shortage LIMIT 5));
SELECT count(*) FROM aggregate_worker_shortage
WHERE body @@@ paradedb.all()
  AND hash = ANY(ARRAY(SELECT hash FROM aggregate_worker_shortage LIMIT 5));
DROP TABLE aggregate_worker_shortage;
RESET max_parallel_workers;
RESET max_parallel_workers_per_gather;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET min_parallel_table_scan_size;
RESET parallel_leader_participation;

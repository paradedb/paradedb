DROP TABLE IF EXISTS partopn;
CREATE TABLE partopn (
    id bigint,
    text text,
    uuid uuid
);
SET max_parallel_maintenance_workers = 4;
INSERT INTO partopn (id, text, uuid) SELECT x, md5(x::text), pg_catalog.gen_random_uuid() FROM generate_series(1, 1000000) x;
CREATE INDEX idxpartopn ON partopn USING bm25 (id, text, uuid) WITH (key_field='id', text_fields = '{
  "text": { "tokenizer": { "type": "keyword" }, "fast": true },
  "uuid": { "tokenizer": { "type": "keyword" }, "fast": true }
}');
ANALYZE partopn;

SELECT count(*) FROM paradedb.index_info('idxpartopn');

SET max_parallel_workers = 2;
SET max_parallel_workers_per_gather = 2;

--
-- order by an integer (id)
--
SET parallel_leader_participation = true;
EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF) SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY id LIMIT 5;
SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY id LIMIT 5;

SET parallel_leader_participation = false;
EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF)  SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY id LIMIT 5;
SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY id LIMIT 5;

--
-- order by a text column (text)
--
SET parallel_leader_participation = true;
EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF) SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY text LIMIT 5;
SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY text LIMIT 5;

SET parallel_leader_participation = false;
EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF)  SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY text LIMIT 5;
SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY text LIMIT 5;

--
-- now with no workers
--

SET max_parallel_workers = 0;
SET max_parallel_workers_per_gather = 0;

--
-- order by an integer (id)
--
SET parallel_leader_participation = true;
EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF) SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY id LIMIT 5;
SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY id LIMIT 5;

SET parallel_leader_participation = false;
EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF)  SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY id LIMIT 5;
SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY id LIMIT 5;

--
-- order by a text column (text)
--
SET parallel_leader_participation = true;
EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF) SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY text LIMIT 5;
SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY text LIMIT 5;

SET parallel_leader_participation = false;
EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, BUFFERS OFF, TIMING OFF, SUMMARY OFF)  SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY text LIMIT 5;
SELECT id, text FROM partopn WHERE id @@@ paradedb.all() ORDER BY text LIMIT 5;


-- NB:  can't order by the `uuid` column because its values are random


RESET ALL;
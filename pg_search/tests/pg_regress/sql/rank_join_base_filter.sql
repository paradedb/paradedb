-- A top-K scored join whose leaves carry a single-table filter (a row-level security policy
-- or a plain WHERE on an indexed column) must still plan RankJoinExec, apply the filter on
-- both the sorted-access stream and the point lookups, and return the rows the hash join
-- path returns.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_rank_join = on;
SET paradedb.rank_join_min_rows = 0;
SET paradedb.global_mutable_segment_rows = 0;

DROP TABLE IF EXISTS rjf_jobs CASCADE;
DROP TABLE IF EXISTS rjf_clients CASCADE;

CREATE TABLE rjf_clients (
    id INT PRIMARY KEY,
    division INT NOT NULL,
    name TEXT NOT NULL
) WITH (autovacuum_enabled = false);

CREATE TABLE rjf_jobs (
    id INT PRIMARY KEY,
    division INT NOT NULL,
    client_id INT NOT NULL,
    body TEXT NOT NULL,
    note TEXT
) WITH (autovacuum_enabled = false);

CREATE INDEX rjf_clients_bm25 ON rjf_clients USING bm25 (id, division, name);
CREATE INDEX rjf_jobs_bm25 ON rjf_jobs USING bm25 (id, division, client_id, body);

-- Term frequencies of 'zebra' decide the ranking. Division 1 rows are visible to the
-- restricted role, division 2 rows are not.
INSERT INTO rjf_clients VALUES
    (1, 1, 'alpha corp'),
    (2, 1, 'beta corp'),
    (3, 2, 'gamma zebra'),
    (4, 2, 'delta corp');

INSERT INTO rjf_jobs VALUES
    (1, 1, 1, 'zebra crossing crossing crossing crossing', 'a'),
    (2, 2, 2, 'zebra zebra zebra stripes crossing', 'a'),
    (3, 1, 3, 'noise crossing', 'a'),
    (4, 2, 4, 'nothing here', 'a'),
    (5, 1, 2, 'zebra plain crossing', 'a'),
    (6, 2, 1, 'zebra zebra zebra zebra crossing crossing', 'a'),
    (7, 1, 4, 'noise words crossing', 'a'),
    (8, 2, 3, 'noise', 'a');

-- The roles are cluster-wide, so a run that stopped before the cleanup leaves them behind.
DROP ROLE IF EXISTS rjf_user;
DROP ROLE IF EXISTS rjf_const_user;
DROP ROLE IF EXISTS rjf_range_user;
CREATE ROLE rjf_user NOLOGIN;
CREATE ROLE rjf_const_user NOLOGIN;
CREATE ROLE rjf_range_user NOLOGIN;
GRANT SELECT ON rjf_jobs, rjf_clients TO rjf_user, rjf_const_user, rjf_range_user;

ALTER TABLE rjf_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE rjf_clients ENABLE ROW LEVEL SECURITY;
-- Policy with a subselect: the predicate reaches the leaf as a Postgres expression.
CREATE POLICY rjf_jobs_user ON rjf_jobs TO rjf_user
    USING (division = ANY ((SELECT string_to_array(current_setting('rjf.divisions', true), ','))::int[]));
CREATE POLICY rjf_clients_user ON rjf_clients TO rjf_user
    USING (division = ANY ((SELECT string_to_array(current_setting('rjf.divisions', true), ','))::int[]));
-- Policy with a constant list: the predicate reaches the leaf as a plain term set.
CREATE POLICY rjf_jobs_const ON rjf_jobs TO rjf_const_user USING (division = ANY ('{1}'::int[]));
CREATE POLICY rjf_clients_const ON rjf_clients TO rjf_const_user USING (division = ANY ('{1}'::int[]));
-- Policy with a `@@@` query: one range for each run of visible division numbers, as an OR in a
-- query string built at execution.
CREATE POLICY rjf_jobs_range ON rjf_jobs TO rjf_range_user
    USING (id @@@ pdb.parse((SELECT string_agg(format('division:[%s TO %s]', lower(r), upper(r) - 1), ' OR ')
                              FROM unnest(coalesce(nullif(current_setting('rjf.division_ranges', true), ''), '{}')::int4multirange) AS r)));
CREATE POLICY rjf_clients_range ON rjf_clients TO rjf_range_user USING (true);

-- ============================================================
-- Baseline: superuser bypasses RLS, so every row competes.
-- ============================================================
SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- ============================================================
-- Test 1: RLS policy with a subselect keeps RankJoinExec.
-- Only jobs 1 and 5 join to a visible client; job 3's client is in division 2.
-- ============================================================
SET rjf.divisions = '1';
SET ROLE rjf_user;

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- The hash join path for the same role returns the same rows, in the same order, with the
-- same scores: a policy adds nothing to pdb.score in either plan.
SET paradedb.enable_rank_join = off;
SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;
SET paradedb.enable_rank_join = on;

-- A client that only the other side matches is still joined (job 5 via client 2 has no
-- name match), and a visible job whose client is hidden is not (job 3 via client 3).
SET rjf.divisions = '2';
SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- An empty division list matches nothing on either side.
SET rjf.divisions = '';
SELECT j.id AS job, c.id AS client
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

RESET ROLE;
RESET rjf.divisions;

-- ============================================================
-- Test 2: RLS policy with a constant list keeps RankJoinExec.
-- ============================================================
SET ROLE rjf_const_user;

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

RESET ROLE;

-- ============================================================
-- Test 3: a plain single-table WHERE with a constant list keeps RankJoinExec.
-- ============================================================
SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra') AND j.division = ANY ('{2}'::int[])
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra') AND j.division = ANY ('{2}'::int[])
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- ============================================================
-- Test 4: a single-table predicate that scores keeps the hash join, so its BM25 still counts.
-- 'crossing' frequencies are chosen so dropping that score would swap jobs 1 and 5.
-- ============================================================
SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra') AND j.body @@@ 'crossing'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra') AND j.body @@@ 'crossing'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- ============================================================
-- Test 5: an OR of two constant filters scores once per matching branch, so job 1, which
-- matches both, outranks job 8, which matches one. The rank join would score both 0.0 and
-- swap them, so it declines and the hash join keeps the branch count.
-- ============================================================
SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND (j.division = ANY ('{1}'::int[]) OR j.client_id = ANY ('{1,3}'::int[]))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND (j.division = ANY ('{1}'::int[]) OR j.client_id = ANY ('{1,3}'::int[]))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- The same OR with each branch at ::pdb.const(0) already scores 0.0 in every plan, so it
-- keeps RankJoinExec, and the hash join returns the same rows, order and scores.
SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND (j.division @@@ pdb.term(1)::pdb.const(0) OR j.client_id @@@ pdb.term_set(ARRAY[1, 3])::pdb.const(0))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND (j.division @@@ pdb.term(1)::pdb.const(0) OR j.client_id @@@ pdb.term_set(ARRAY[1, 3])::pdb.const(0))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

SET paradedb.enable_rank_join = off;
SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND (j.division @@@ pdb.term(1)::pdb.const(0) OR j.client_id @@@ pdb.term_set(ARRAY[1, 3])::pdb.const(0))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;
SET paradedb.enable_rank_join = on;

-- ============================================================
-- Test 6: a filter the index cannot answer keeps the hash join.
-- ============================================================
SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra') AND j.note = 'a'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

-- ============================================================
-- Test 7: scalar equality on an untokenized column is a filter like a one-element list.
-- ============================================================
SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra') AND j.division = 2
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra') AND j.division = 2
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- ============================================================
-- Test 8: an explicit term search is not a pushed filter, so it keeps the hash join.
-- ============================================================
SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra') AND j.body @@@ pdb.term('crossing')
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

-- ============================================================
-- Test 9: a constant-score wrapper around a text search is still a scored search.
-- ============================================================
SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra') AND j.body @@@ pdb.term('crossing')::pdb.const(7.0)
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

-- ============================================================
-- Test 10: a policy written as a @@@ query of ranges keeps RankJoinExec, because a policy is
-- lowered at score 0.0 whatever its shape. Every client is visible to this role, so job 3
-- joins through its division 2 client.
-- ============================================================
SET rjf.division_ranges = '{[1,2)}';
SET ROLE rjf_range_user;

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- Two runs of divisions see every job, like the superuser.
SET rjf.division_ranges = '{[1,2),[2,3)}';
SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- The hash join scores the same rows the same.
SET paradedb.enable_rank_join = off;
SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;
SET paradedb.enable_rank_join = on;

RESET ROLE;
RESET rjf.division_ranges;

-- ============================================================
-- Test 11: a policy adds nothing to pdb.score in a single-table search either: a restricted
-- role scores a job as the superuser does, while a WHERE filter the user writes still adds
-- its own score.
-- ============================================================
SELECT id, pdb.score(id) AS score FROM rjf_jobs WHERE body @@@ 'zebra' ORDER BY id;
SET ROLE rjf_const_user;
SELECT id, pdb.score(id) AS score FROM rjf_jobs WHERE body @@@ 'zebra' ORDER BY id;
RESET ROLE;
SELECT id, pdb.score(id) AS score FROM rjf_jobs WHERE body @@@ 'zebra' AND division = 1 ORDER BY id;

-- ============================================================
-- Test 12: a @@@ query whose score is constant (pdb.range, pdb.term_set, pdb.exists) is a
-- filter the user wrote with the search operator. It keeps RankJoinExec like the plain SQL
-- range it is equivalent to. An OR of two such ranges scores per matching branch and declines,
-- as in Test 5.
-- ============================================================
SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND j.division @@@ pdb.range(int4range(2, 3))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND j.division @@@ pdb.range(int4range(2, 3))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- The hash join adds the range's constant score to every row it keeps, so the rows and their
-- order are the same and only the job score differs by that constant.
SET paradedb.enable_rank_join = off;
SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND j.division @@@ pdb.range(int4range(2, 3))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;
SET paradedb.enable_rank_join = on;

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND (j.division @@@ pdb.range(int4range(1, 2)) OR j.division @@@ pdb.range(int4range(2, 3)))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND (j.division @@@ pdb.range(int4range(1, 2)) OR j.division @@@ pdb.range(int4range(2, 3)))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- ============================================================
-- Test 13: with MPP enabled but inputs that are not range co-partitioned, a filtered join
-- still plans and runs the single-task RankJoinExec (mode=CollectLeft).
-- ============================================================
SET paradedb.mpp_min_rows = 0;
SET max_parallel_workers_per_gather = 3;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET ROLE rjf_const_user;

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

RESET ROLE;
RESET paradedb.mpp_min_rows;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;

-- ============================================================
-- Test 14: range co-partitioned inputs keep RankJoinExec (mode=Partitioned) under
-- DistributedExec, with the policy applied on every task's leaf. Extra rows let the join
-- key split into two partitions; the indexes are rebuilt with partition keys.
-- ============================================================
INSERT INTO rjf_clients
SELECT g, 1 + g % 2, CASE WHEN g % 7 = 0 THEN 'zebra client' ELSE 'corp' END
FROM generate_series(100, 160) g;
INSERT INTO rjf_jobs
SELECT g, 1 + g % 2, 100 + g % 61, CASE WHEN g % 5 = 0 THEN 'zebra job' ELSE 'noise' END, 'a'
FROM generate_series(200, 320) g;
DROP INDEX rjf_clients_bm25, rjf_jobs_bm25;
SET max_parallel_maintenance_workers = 0;
CREATE INDEX rjf_clients_bm25 ON rjf_clients USING bm25 (id, division, name)
WITH (target_segment_count = 3, partition_by = 'id');
CREATE INDEX rjf_jobs_bm25 ON rjf_jobs USING bm25 (id, division, client_id, body)
WITH (target_segment_count = 3, partition_by = 'client_id,id');
ANALYZE rjf_clients, rjf_jobs;

SET paradedb.enable_range_partitioned_join = on;
SET paradedb.mpp_min_rows = 0;
SET max_parallel_workers_per_gather = 3;
SET max_parallel_workers = 8;
SET min_parallel_table_scan_size = 0;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET ROLE rjf_const_user;

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- The distributed hash join returns the same rows.
SET paradedb.enable_rank_join = off;
SELECT j.id AS job, c.id AS client
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;
SET paradedb.enable_rank_join = on;

RESET ROLE;
RESET paradedb.enable_range_partitioned_join;
RESET paradedb.mpp_min_rows;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;

-- ============================================================
-- Cleanup
-- ============================================================
DROP TABLE rjf_jobs CASCADE;
DROP TABLE rjf_clients CASCADE;
DROP ROLE rjf_user;
DROP ROLE rjf_const_user;
DROP ROLE rjf_range_user;

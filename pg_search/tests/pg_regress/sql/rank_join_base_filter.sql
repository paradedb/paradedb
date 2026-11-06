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
CREATE ROLE rjf_user NOLOGIN;
CREATE ROLE rjf_const_user NOLOGIN;
GRANT SELECT ON rjf_jobs, rjf_clients TO rjf_user, rjf_const_user;

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

-- The hash join path for the same role returns the same rows in the same order. Its scores
-- include the constant score of the policy term set (+1 per table).
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
-- Test 5: an OR of two constant filters scores once per matching branch, so it keeps the
-- hash join and the branch count still counts.
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
-- Test 10: with MPP enabled, a filtered join still plans and runs the single-task
-- RankJoinExec (it needs one task for its in-memory dynamic filters).
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
-- Cleanup
-- ============================================================
DROP TABLE rjf_jobs CASCADE;
DROP TABLE rjf_clients CASCADE;
DROP ROLE rjf_user;
DROP ROLE rjf_const_user;

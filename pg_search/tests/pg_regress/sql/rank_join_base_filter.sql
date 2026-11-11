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
DROP ROLE IF EXISTS rjf_eq_user;
DROP ROLE IF EXISTS rjf_or_user;
DROP ROLE IF EXISTS rjf_range_user;
CREATE ROLE rjf_user NOLOGIN;
CREATE ROLE rjf_const_user NOLOGIN;
CREATE ROLE rjf_eq_user NOLOGIN;
CREATE ROLE rjf_or_user NOLOGIN;
CREATE ROLE rjf_range_user NOLOGIN;
GRANT SELECT ON rjf_jobs, rjf_clients TO rjf_user, rjf_const_user, rjf_eq_user, rjf_or_user, rjf_range_user;

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
-- Policy with a plain equality: it joins an equivalence class with any user equality on the
-- same constant.
CREATE POLICY rjf_jobs_eq ON rjf_jobs TO rjf_eq_user USING (division = 1);
-- Policy with a SQL OR: each arm reaches the planner as its own RestrictInfo.
CREATE POLICY rjf_jobs_or ON rjf_jobs TO rjf_or_user USING (division = 1 OR client_id = 3);
CREATE POLICY rjf_clients_or ON rjf_clients TO rjf_or_user USING (true);
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
-- Test 13: a policy equality and a user equality on the same constant share one equivalence
-- class, and PostgreSQL regenerates both clauses at the policy's level. The user's
-- client_id = 1 still scores as it does for the superuser, and only division = 1 is lowered
-- as the policy. With client_id = 2 no class forms, so that case is the control.
-- ============================================================
SELECT id, pdb.score(id) AS score FROM rjf_jobs WHERE body @@@ 'zebra' AND client_id = 1 ORDER BY id;
SELECT id, pdb.score(id) AS score FROM rjf_jobs WHERE body @@@ 'zebra' AND client_id = 2 ORDER BY id;
SET ROLE rjf_eq_user;
SELECT id, pdb.score(id) AS score FROM rjf_jobs WHERE body @@@ 'zebra' AND client_id = 1 ORDER BY id;
SELECT id, pdb.score(id) AS score FROM rjf_jobs WHERE body @@@ 'zebra' AND client_id = 2 ORDER BY id;
SELECT explain_redact_oid($$
SELECT id FROM rjf_jobs WHERE body @@@ 'zebra' AND client_id = 1 ORDER BY id
$$);
-- A user equality that repeats the policy's own is one regenerated clause for both; it keeps
-- the user's score, so the row scores as it does for the superuser (Test 11).
SELECT id, pdb.score(id) AS score FROM rjf_jobs WHERE body @@@ 'zebra' AND division = 1 ORDER BY id;
RESET ROLE;

-- A partition's copy of the regenerated clause has no equivalence-class link of its own, so
-- the parent's clause decides: the user's client_id = 1 scores as for the superuser here too.
CREATE TABLE rjf_jobs_part (
    id INT NOT NULL,
    division INT NOT NULL,
    client_id INT NOT NULL,
    body TEXT NOT NULL
) PARTITION BY LIST (division);
CREATE TABLE rjf_jobs_part_1 PARTITION OF rjf_jobs_part FOR VALUES IN (1);
CREATE TABLE rjf_jobs_part_2 PARTITION OF rjf_jobs_part FOR VALUES IN (2);
INSERT INTO rjf_jobs_part SELECT id, division, client_id, body FROM rjf_jobs;
CREATE INDEX rjf_jobs_part_bm25 ON rjf_jobs_part USING bm25 (id, division, client_id, body);
GRANT SELECT ON rjf_jobs_part TO rjf_eq_user;
ALTER TABLE rjf_jobs_part ENABLE ROW LEVEL SECURITY;
CREATE POLICY rjf_jobs_part_eq ON rjf_jobs_part TO rjf_eq_user USING (division = 1);

SELECT id, pdb.score(id) AS score FROM rjf_jobs_part WHERE body @@@ 'zebra' AND client_id = 1 ORDER BY id;
SET ROLE rjf_eq_user;
SELECT id, pdb.score(id) AS score FROM rjf_jobs_part WHERE body @@@ 'zebra' AND client_id = 1 ORDER BY id;
SELECT explain_redact_oid($$
SELECT id FROM rjf_jobs_part WHERE body @@@ 'zebra' AND client_id = 1 ORDER BY id
$$);
RESET ROLE;
DROP TABLE rjf_jobs_part;

-- ============================================================
-- Test 14: a policy written as a SQL OR keeps RankJoinExec: each arm is lowered at 0.0 once,
-- so the OR already scores 0.0 in every plan, and the hash join agrees on rows and scores.
-- ============================================================
SET ROLE rjf_or_user;
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

SET paradedb.enable_rank_join = off;
SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;
SET paradedb.enable_rank_join = on;
RESET ROLE;

-- ============================================================
-- Test 15: with MPP enabled but inputs that are not range co-partitioned, a filtered join
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
-- Test 16: range co-partitioned inputs keep RankJoinExec (mode=Partitioned) under
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
-- Test 17: range base filters use inverted-index dictionary statistics rather than Tantivy's
-- 10% fast-field heuristic, declining RankJoinExec below rank_join_min_rows (including after
-- InitPlan solving in pdb.parse range policies) while keeping RankJoinExec on broad ranges and
-- returning identical rows/scores across multi-segment indexes.
-- ============================================================
SET max_parallel_workers_per_gather = 0;
INSERT INTO rjf_jobs VALUES
    (400, 99, 100, 'zebra rare', 'a'),
    (401, 99, 105, 'noise', 'a');

-- Under rank_join_min_rows = 10, division 99 has 2 jobs (< 10, whereas 10% of 131 = 13 >= 10),
-- so pdb.range, split SQL range bounds, and InitPlan pdb.parse range policies all decline
-- RankJoinExec; division 1 has 65 jobs (>= 10, with 65 clients >= 10) and keeps RankJoinExec.
SET paradedb.rank_join_min_rows = 10;

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND j.division @@@ pdb.range(int4range(99, 100))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND j.division >= 99 AND j.division < 100
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND j.division @@@ pdb.range(int4range(1, 2))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SET rjf.division_ranges = '{[99,100)}';
SET ROLE rjf_range_user;
SELECT bool_or(line LIKE '%RankJoinExec%') AS uses_rank_join
FROM explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$, 'ANALYZE, COSTS OFF, TIMING OFF, SUMMARY OFF') AS line;

SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

-- Forcing RankJoinExec (rank_join_min_rows = 0) across segments where 3 of 4 segments have
-- zero matching base docs returns the exact same rows and scores.
RESET ROLE;
SET paradedb.rank_join_min_rows = 0;
SET ROLE rjf_range_user;
SELECT j.id AS job, c.id AS client, pdb.score(j.id) AS job_score, pdb.score(c.id) AS client_score
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10;

RESET ROLE;
SET paradedb.rank_join_min_rows = 10;
SET rjf.division_ranges = '{[1,2)}';
SET ROLE rjf_range_user;
SELECT bool_or(line LIKE '%RankJoinExec%') AS uses_rank_join
FROM explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE j.body @@@ 'zebra' OR c.name @@@ 'zebra'
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$, 'ANALYZE, COSTS OFF, TIMING OFF, SUMMARY OFF') AS line;

RESET ROLE;
RESET rjf.division_ranges;

-- Under rank_join_min_rows = 1, division 99 has 2 jobs (>= 1), but selectivity 2/130 = 0.0154
-- is below rank_join_unscored_selectivity_threshold (0.025), so Gate 3 declines RankJoinExec;
-- lowering rank_join_unscored_selectivity_threshold to 0.01 admits RankJoinExec and scales only
-- the unfiltered partner leaf's lazy_sort window (c: 1300, j: 20).
SET paradedb.rank_join_min_rows = 1;

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND j.division @@@ pdb.range(int4range(99, 100))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

SET paradedb.rank_join_unscored_selectivity_threshold = 0.01;

SELECT explain_redact_oid($$
SELECT j.id, c.id
FROM rjf_jobs j JOIN rjf_clients c ON c.id = j.client_id
WHERE (j.body @@@ 'zebra' OR c.name @@@ 'zebra')
  AND j.division @@@ pdb.range(int4range(99, 100))
ORDER BY pdb.score(j.id) + pdb.score(c.id) DESC, j.id
LIMIT 10
$$);

RESET paradedb.rank_join_unscored_selectivity_threshold;
SET paradedb.rank_join_min_rows = 0;

-- ============================================================
-- Cleanup
-- ============================================================
DROP TABLE rjf_jobs CASCADE;
DROP TABLE rjf_clients CASCADE;
DROP ROLE rjf_user;
DROP ROLE rjf_const_user;
DROP ROLE rjf_eq_user;
DROP ROLE rjf_or_user;
DROP ROLE rjf_range_user;

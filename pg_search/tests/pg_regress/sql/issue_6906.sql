-- auto_explain prints plans from its ExecutorEnd hook, where no snapshot is
-- active. Without ANALYZE, the Aggregate Scan and Join Scan rebuild their
-- DataFusion plans to print them, and that rebuild needs a snapshot.
-- https://github.com/paradedb/paradedb/issues/6906

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;

CREATE TABLE issue_6906_t (id bigint PRIMARY KEY, cik bigint, body text);
INSERT INTO issue_6906_t SELECT g, g % 7, 'letter of intent ' || g FROM generate_series(1, 100) g;
CREATE INDEX issue_6906_t_idx ON issue_6906_t USING paradedb (id, body, cik);

CREATE TABLE issue_6906_f (cik bigint PRIMARY KEY, name text);
INSERT INTO issue_6906_f SELECT g, 'n' || g FROM generate_series(0, 6) g;
CREATE INDEX issue_6906_f_idx ON issue_6906_f USING paradedb (cik, name);

EXPLAIN (COSTS OFF)
SELECT cik, count(*) FROM issue_6906_t GROUP BY cik ORDER BY 2 DESC, 1 LIMIT 1;

EXPLAIN (COSTS OFF)
SELECT t.id, f.name FROM issue_6906_t t JOIN issue_6906_f f ON f.cik = t.cik
WHERE t.body @@@ 'intent' AND f.name @@@ 'n1'
ORDER BY t.id LIMIT 5;

LOAD 'auto_explain';
SET auto_explain.log_min_duration = 0;

SELECT cik, count(*) FROM issue_6906_t GROUP BY cik ORDER BY 2 DESC, 1 LIMIT 1;

SELECT t.id, f.name FROM issue_6906_t t JOIN issue_6906_f f ON f.cik = t.cik
WHERE t.body @@@ 'intent' AND f.name @@@ 'n1'
ORDER BY t.id LIMIT 5;

RESET auto_explain.log_min_duration;
RESET max_parallel_workers_per_gather;
DROP TABLE issue_6906_t, issue_6906_f;

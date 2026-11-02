CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_6929_t (id int PRIMARY KEY, body text, grp int4, b int8);
INSERT INTO issue_6929_t VALUES
    (1, 'x', 1, 9007199254740993),
    (2, 'x', 1, 9007199254740993),
    (3, 'x', 2, 5),
    (4, 'x', 2, NULL);
CREATE INDEX issue_6929_t_idx ON issue_6929_t USING paradedb (id, body, grp, b);

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;

-- sum(int8) runs in the DataFusion aggregate backend, not Tantivy's f64 sum
EXPLAIN (COSTS OFF)
SELECT sum(b) FROM issue_6929_t WHERE body @@@ 'x';

SELECT sum(b) FROM issue_6929_t WHERE body @@@ 'x';
SELECT grp, sum(b) FROM issue_6929_t WHERE body @@@ 'x' GROUP BY grp ORDER BY grp;
SELECT sum(b) * 2 AS doubled FROM issue_6929_t WHERE body @@@ 'x';
SELECT grp FROM issue_6929_t WHERE body @@@ 'x'
GROUP BY grp HAVING sum(b) = 18014398509481986 ORDER BY grp;

-- Parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT sum(b) FROM issue_6929_t WHERE body @@@ 'x';
SELECT grp, sum(b) FROM issue_6929_t WHERE body @@@ 'x' GROUP BY grp ORDER BY grp;
SELECT sum(b) * 2 AS doubled FROM issue_6929_t WHERE body @@@ 'x';
SELECT grp FROM issue_6929_t WHERE body @@@ 'x'
GROUP BY grp HAVING sum(b) = 18014398509481986 ORDER BY grp;

RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
DROP TABLE issue_6929_t;

-- Regression for #7264: nested score expressions must deparse without placeholder errors.
SET max_parallel_workers_per_gather = 0;

CREATE TABLE score_expression_sa (id bigint PRIMARY KEY, body text);
INSERT INTO score_expression_sa SELECT i, 'hello world' FROM generate_series(1, 5) i;
CREATE INDEX score_expression_sa_idx ON score_expression_sa USING bm25 (id, body);

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT count(*) FILTER (WHERE id + pdb.score(id)::bigint > 0)
FROM score_expression_sa
WHERE body ||| 'hello';

RESET max_parallel_workers_per_gather;
DROP TABLE score_expression_sa;

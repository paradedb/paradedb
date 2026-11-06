-- MIN and MAX over bigint must stay exact past 2^53, including COALESCE arguments.
-- Tantivy computes them in f64, so they run on the DataFusion aggregate backend.
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_7263_t (id bigint PRIMARY KEY, grp int4, b int8, small int4, body text);
INSERT INTO issue_7263_t VALUES
    (1234567890123456789, 1, 9223372036854775807,  1, 'payment failed'),
    (1234567890123456001, 1, -9223372036854775808, 2, 'payment ok'),
    (1234567890123456003, 2, 9007199254740993,     3, 'payment late'),
    (1234567890123456005, 2, -9007199254740993,    4, 'payment held'),
    (1234567890123456007, 3, NULL,                 5, 'payment void');
CREATE INDEX issue_7263_t_idx ON issue_7263_t USING paradedb (id, grp, b, small, body);

CREATE TABLE issue_7263_u (id bigint PRIMARY KEY, grp int4, v int8, body text);
INSERT INTO issue_7263_u VALUES (1, 1, 9007199254740993, 'refund'), (2, 1, NULL, 'refund');
CREATE INDEX issue_7263_u_idx ON issue_7263_u USING paradedb (id, grp, v, body);

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;

-- MIN and MAX of a bigint run in the DataFusion aggregate backend
EXPLAIN (COSTS OFF)
SELECT MAX(id), MIN(id) FROM issue_7263_t WHERE body ||| 'payment';

-- COALESCE stays pushed down, and applies its default
EXPLAIN (COSTS OFF)
SELECT MAX(COALESCE(b, 0)) FROM issue_7263_t WHERE body ||| 'payment';

-- Aggregates that read a COALESCE on Tantivy stay pushed down beside it
EXPLAIN (COSTS OFF)
SELECT COUNT(COALESCE(b, 0)), SUM(COALESCE(b, 0)), MAX(COALESCE(b, 0))
FROM issue_7263_t WHERE body ||| 'payment';

-- int4 is exact in f64, so it stays on Tantivy
EXPLAIN (COSTS OFF)
SELECT MAX(small), MIN(small) FROM issue_7263_t WHERE body ||| 'payment';

SELECT MAX(id), MIN(id) FROM issue_7263_t WHERE body ||| 'payment';
SELECT MAX(b), MIN(b) FROM issue_7263_t WHERE body ||| 'payment';
SELECT * FROM issue_7263_t
WHERE id = (SELECT MAX(id) FROM issue_7263_t WHERE body ||| 'payment');
SELECT grp, MAX(id), MIN(b) FROM issue_7263_t WHERE body ||| 'payment' GROUP BY grp ORDER BY grp;
SELECT MAX(id), COUNT(*) FROM issue_7263_t WHERE body ||| 'payment';
SELECT MAX(id) FILTER (WHERE grp > 1) FROM issue_7263_t WHERE body ||| 'payment';
SELECT grp FROM issue_7263_t WHERE body ||| 'payment'
GROUP BY grp HAVING MAX(id) = 1234567890123456789 ORDER BY grp;
SELECT MAX(id) FROM issue_7263_t WHERE body ||| 'nomatch';

-- COALESCE: a NULL reads as the default, and an exact default survives
SELECT MAX(COALESCE(b, 0)), MIN(COALESCE(b, 0)) FROM issue_7263_t WHERE body ||| 'void OR late';
SELECT MAX(COALESCE(b, 1234567890123456789)) FROM issue_7263_t WHERE body ||| 'void';
SELECT MIN(COALESCE(id, 5)) FROM issue_7263_t WHERE body ||| 'payment';
SELECT grp, MAX(COALESCE(b, 7)) FROM issue_7263_t WHERE body ||| 'payment' GROUP BY grp ORDER BY grp;
SELECT COUNT(COALESCE(b, 0)), SUM(COALESCE(b, 0)), MAX(COALESCE(b, 0))
FROM issue_7263_t WHERE body ||| 'payment';
SELECT SUM(COALESCE(id, 0)) FROM issue_7263_t WHERE body ||| 'payment';

-- The default also applies to rows a LEFT JOIN leaves unmatched
EXPLAIN (COSTS OFF)
SELECT MIN(COALESCE(u.v, -1)), MAX(COALESCE(u.v, -1)), COUNT(COALESCE(u.v, -1))
FROM issue_7263_t t LEFT JOIN issue_7263_u u ON t.grp = u.grp WHERE t.body ||| 'payment';
SELECT MIN(COALESCE(u.v, -1)), MAX(COALESCE(u.v, -1)), COUNT(COALESCE(u.v, -1))
FROM issue_7263_t t LEFT JOIN issue_7263_u u ON t.grp = u.grp WHERE t.body ||| 'payment';

-- Parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT MAX(id), MIN(id) FROM issue_7263_t WHERE body ||| 'payment';
SELECT MAX(b), MIN(b) FROM issue_7263_t WHERE body ||| 'payment';
SELECT * FROM issue_7263_t
WHERE id = (SELECT MAX(id) FROM issue_7263_t WHERE body ||| 'payment');
SELECT grp, MAX(id), MIN(b) FROM issue_7263_t WHERE body ||| 'payment' GROUP BY grp ORDER BY grp;
SELECT MAX(id), COUNT(*) FROM issue_7263_t WHERE body ||| 'payment';
SELECT MAX(id) FILTER (WHERE grp > 1) FROM issue_7263_t WHERE body ||| 'payment';
SELECT grp FROM issue_7263_t WHERE body ||| 'payment'
GROUP BY grp HAVING MAX(id) = 1234567890123456789 ORDER BY grp;
SELECT MAX(id) FROM issue_7263_t WHERE body ||| 'nomatch';
SELECT MAX(COALESCE(b, 0)), MIN(COALESCE(b, 0)) FROM issue_7263_t WHERE body ||| 'void OR late';
SELECT MAX(COALESCE(b, 1234567890123456789)) FROM issue_7263_t WHERE body ||| 'void';
SELECT MIN(COALESCE(id, 5)) FROM issue_7263_t WHERE body ||| 'payment';
SELECT grp, MAX(COALESCE(b, 7)) FROM issue_7263_t WHERE body ||| 'payment' GROUP BY grp ORDER BY grp;
SELECT COUNT(COALESCE(b, 0)), SUM(COALESCE(b, 0)), MAX(COALESCE(b, 0))
FROM issue_7263_t WHERE body ||| 'payment';
SELECT SUM(COALESCE(id, 0)) FROM issue_7263_t WHERE body ||| 'payment';
SELECT MIN(COALESCE(u.v, -1)), MAX(COALESCE(u.v, -1)), COUNT(COALESCE(u.v, -1))
FROM issue_7263_t t LEFT JOIN issue_7263_u u ON t.grp = u.grp WHERE t.body ||| 'payment';

RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
DROP TABLE issue_7263_u;
DROP TABLE issue_7263_t;

-- MPP scaling micro-benchmark setup.
-- Pass row count via psql -v rows=1000000

\set ON_ERROR_STOP on

DROP TABLE IF EXISTS mpp_bench_child CASCADE;
DROP TABLE IF EXISTS mpp_bench CASCADE;

-- Parent table: indexed for BM25, search predicate goes here.
CREATE TABLE mpp_bench (
    id BIGINT PRIMARY KEY,
    category INT NOT NULL,
    user_id BIGINT NOT NULL,
    score INT NOT NULL,
    body TEXT NOT NULL
);

INSERT INTO mpp_bench (id, category, user_id, score, body)
SELECT
    g,
    g % 10,
    g / 10,
    (random() * 1000)::int,
    CASE WHEN g % 10 < 3 THEN 'term filler text ' || (g % 1000)::text
         ELSE 'other filler ' || (g % 1000)::text
    END
FROM generate_series(1, :rows) g;

CREATE INDEX mpp_bench_idx ON mpp_bench USING bm25 (id, body, category, user_id, score)
WITH (key_field='id', numeric_fields='{"user_id":{"fast":true},"category":{"fast":true},"score":{"fast":true}}');

-- Child table: 5x rows of parent, joined by parent_id. Forces JoinScan / MPP.
CREATE TABLE mpp_bench_child (
    cid BIGINT PRIMARY KEY,
    parent_id BIGINT NOT NULL,
    amount INT NOT NULL,
    note TEXT NOT NULL
);

INSERT INTO mpp_bench_child (cid, parent_id, amount, note)
SELECT
    g,
    ((g - 1) / 5) + 1,   -- 5 children per parent
    (random() * 100)::int,
    'child note ' || g::text
FROM generate_series(1, :rows * 5) g;

CREATE INDEX mpp_bench_child_idx ON mpp_bench_child USING bm25 (cid, parent_id, amount, note)
WITH (key_field='cid', numeric_fields='{"parent_id":{"fast":true},"amount":{"fast":true}}');

ANALYZE mpp_bench;
ANALYZE mpp_bench_child;

SELECT pg_size_pretty(pg_total_relation_size('mpp_bench')) AS parent_size,
       pg_size_pretty(pg_total_relation_size('mpp_bench_child')) AS child_size,
       (SELECT count(*) FROM mpp_bench) AS parent_rows,
       (SELECT count(*) FROM mpp_bench_child) AS child_rows;

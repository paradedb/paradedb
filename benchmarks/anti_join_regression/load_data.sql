\set ON_ERROR_STOP on

-- Four committed insert waves create multiple independently claimable BM25
-- segments. The recorded indexes contained six segments; the index settings
-- prevent background merges from collapsing them before the comparison.
INSERT INTO anti_bench_library (id, title, category)
SELECT i,
       CASE
           WHEN i % 11 = 0 THEN 'dragon chronicle volume ' || i
           WHEN i % 7 = 0 THEN 'love story volume ' || i
           ELSE 'general catalog volume ' || i
       END,
       CASE WHEN i % 5 = 0 THEN 'fantasy' ELSE 'fiction' END
FROM generate_series(1, 25000) AS i;

INSERT INTO anti_bench_library (id, title, category)
SELECT i,
       CASE
           WHEN i % 11 = 0 THEN 'dragon chronicle volume ' || i
           WHEN i % 7 = 0 THEN 'love story volume ' || i
           ELSE 'general catalog volume ' || i
       END,
       CASE WHEN i % 5 = 0 THEN 'fantasy' ELSE 'fiction' END
FROM generate_series(25001, 50000) AS i;

INSERT INTO anti_bench_library (id, title, category)
SELECT i,
       CASE
           WHEN i % 11 = 0 THEN 'dragon chronicle volume ' || i
           WHEN i % 7 = 0 THEN 'love story volume ' || i
           ELSE 'general catalog volume ' || i
       END,
       CASE WHEN i % 5 = 0 THEN 'fantasy' ELSE 'fiction' END
FROM generate_series(50001, 75000) AS i;

INSERT INTO anti_bench_library (id, title, category)
SELECT i,
       CASE
           WHEN i % 11 = 0 THEN 'dragon chronicle volume ' || i
           WHEN i % 7 = 0 THEN 'love story volume ' || i
           ELSE 'general catalog volume ' || i
       END,
       CASE WHEN i % 5 = 0 THEN 'fantasy' ELSE 'fiction' END
FROM generate_series(75001, 100000) AS i;

INSERT INTO anti_bench_owned (user_id, item_id)
SELECT 42, i FROM generate_series(1, 18750) AS i;
INSERT INTO anti_bench_owned (user_id, item_id)
SELECT 42, i FROM generate_series(18751, 37500) AS i;
INSERT INTO anti_bench_owned (user_id, item_id)
SELECT 42, i FROM generate_series(37501, 56250) AS i;
INSERT INTO anti_bench_owned (user_id, item_id)
SELECT 42, i FROM generate_series(56251, 75000) AS i;

VACUUM (ANALYZE) anti_bench_library;
VACUUM (ANALYZE) anti_bench_owned;

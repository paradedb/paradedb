\i common/common_setup.sql

-- Exercise dictionary decoding across blocks with consecutive, sparse, duplicate,
-- and NULL ordinals. A non-byte collation keeps sorting above the columnar scan.
SET paradedb.global_mutable_segment_rows = 0;
SET paradedb.enable_aggregate_custom_scan = off;
SET paradedb.planner_warnings = 'off';

CREATE TABLE decode_batches (
    id INT PRIMARY KEY,
    body TEXT,
    label TEXT COLLATE "en-x-icu"
);
CREATE INDEX decode_batches_idx ON decode_batches
USING paradedb (id, (body::pdb.simple('columnar=true')), (label::pdb.literal))
WITH (layer_sizes = '0', background_layer_sizes = '0');
INSERT INTO decode_batches
SELECT i,
       'common' || CASE WHEN i % 97 = 0 THEN ' rare' ELSE '' END ||
       CASE WHEN i % 11 = 0 THEN ' missing' ELSE '' END,
       CASE WHEN i % 11 = 0 THEN NULL WHEN i % 7 = 0 THEN 'duplicate'
            WHEN i % 17 = 0 THEN '' WHEN i % 19 = 0 THEN repeat('é🦀', 20) || i::text
            ELSE repeat(md5(i::text), 4) END
FROM generate_series(1, 8192) i;
INSERT INTO decode_batches
SELECT i,
       'common' || CASE WHEN i % 97 = 0 THEN ' rare' ELSE '' END ||
       CASE WHEN i % 11 = 0 THEN ' missing' ELSE '' END,
       CASE WHEN i % 11 = 0 THEN NULL WHEN i % 7 = 0 THEN 'duplicate'
            WHEN i % 17 = 0 THEN '' WHEN i % 19 = 0 THEN repeat('é🦀', 20) || i::text
            ELSE repeat(md5(i::text), 4) END
FROM generate_series(8193, 16384) i;
ANALYZE decode_batches;

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, label FROM decode_batches
WHERE body ||| 'common' ORDER BY label LIMIT 20000;

WITH actual AS MATERIALIZED (
    SELECT id, label FROM decode_batches
    WHERE body ||| 'common' ORDER BY label LIMIT 20000
)
SELECT count(*) = 16384 AND bool_and(a.label IS NOT DISTINCT FROM d.label) AS dense_correct
FROM actual a JOIN decode_batches d USING (id);

WITH actual AS MATERIALIZED (
    SELECT id, label FROM decode_batches
    WHERE body ||| 'rare' ORDER BY label LIMIT 20000
)
SELECT count(*) = (SELECT count(*) FROM decode_batches WHERE body LIKE '%rare%')
       AND bool_and(a.label IS NOT DISTINCT FROM d.label) AS sparse_correct
FROM actual a JOIN decode_batches d USING (id);

WITH actual AS MATERIALIZED (
    SELECT id, label FROM decode_batches
    WHERE body ||| 'missing' ORDER BY label LIMIT 20000
)
SELECT count(*) = (SELECT count(*) FROM decode_batches WHERE body LIKE '%missing%')
       AND bool_and(label IS NULL) AS nulls_correct
FROM actual;

SELECT count(*) = 0 AS empty_correct FROM (
    SELECT id, label FROM decode_batches
    WHERE body ||| 'nonexistent' ORDER BY label LIMIT 20000
) q;

-- TopK returns addresses out of ordinal order and exercises sparse fetches.
WITH actual AS MATERIALIZED (
    SELECT id, label FROM decode_batches
    WHERE body ||| 'common' ORDER BY id DESC LIMIT 20
)
SELECT count(*) = 20 AND min(id) = 16365 AND max(id) = 16384
       AND bool_and(a.label IS NOT DISTINCT FROM d.label) AS topk_correct
FROM actual a JOIN decode_batches d USING (id);

DROP TABLE decode_batches;
RESET paradedb.global_mutable_segment_rows;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.planner_warnings;
\i common/common_cleanup.sql

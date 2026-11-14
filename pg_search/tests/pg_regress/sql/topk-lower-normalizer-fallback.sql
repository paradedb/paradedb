-- Regression tests for #7054: Top-K over lower(text)::pdb.literal under the C collation.
--  * The explicit pdb.literal normalizer must survive the lower() inference, so the
--    indexed values stay in PostgreSQL's order and Top-K is used.
--  * Results must match PostgreSQL's own sort with the custom scan on and off.
--  * Numeric fast fields keep the Top-K path.

\i common/common_setup.sql

SET max_parallel_workers_per_gather = 0;

DROP TABLE IF EXISTS lower_fallback CASCADE;

CREATE TABLE lower_fallback (
    id integer PRIMARY KEY,
    body text,
    category text COLLATE "C",
    score integer
);

INSERT INTO lower_fallback VALUES
    (1, 'wireless', 'ä', 10),
    (2, 'wireless', 'Å', 20),
    (3, 'wireless', 'apple', 30),
    (4, 'wireless', 'Banana', 40);

CREATE INDEX lower_fallback_raw_idx ON lower_fallback USING paradedb (
    id, body, (lower(category)::pdb.literal)
);

-- 1. Top-K is used for the literal lower() expression.
EXPLAIN (COSTS OFF)
SELECT id FROM lower_fallback
WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id
LIMIT 3;

-- 2. Results with Top-K match PostgreSQL's own sort.
SET paradedb.enable_custom_scan = on;
SELECT id, category FROM lower_fallback
WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id
LIMIT 3;

SET paradedb.enable_custom_scan = off;
SELECT id, category FROM lower_fallback
WHERE body ||| 'wireless'
ORDER BY lower(category) COLLATE "C", id
LIMIT 3;

-- 3. Numeric fast field keeps Top-K.
DROP INDEX lower_fallback_raw_idx;
CREATE INDEX lower_fallback_score_idx ON lower_fallback USING paradedb (
    id, body, score
);
SET paradedb.enable_custom_scan = on;
EXPLAIN (COSTS OFF)
SELECT id FROM lower_fallback
WHERE body ||| 'wireless'
ORDER BY score, id
LIMIT 3;

DROP TABLE lower_fallback;

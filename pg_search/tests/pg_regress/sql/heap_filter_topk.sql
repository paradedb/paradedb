-- Top K by score with heap filters: the heap filters run only on documents that can
-- still enter the top K. Each query is compared with a reference that scores every
-- match of the indexed conditions first and applies the heap conditions in Postgres.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET client_min_messages = warning;
SET paradedb.enable_filter_pushdown TO on;
SET max_parallel_workers_per_gather TO 0;

DROP TABLE IF EXISTS hf_topk CASCADE;
CREATE TABLE hf_topk (
    id INT PRIMARY KEY,
    body TEXT,
    title TEXT,
    price FLOAT8,
    qty INT
);

-- "kiwi" is in a third of the rows, 1 to 4 times, in documents of varying length, so its
-- scores spread over many values; the rows with id % 4 = 3 have it 4 times and score
-- highest. "apple" is in every row, so its scores tie.
INSERT INTO hf_topk (id, body, title, price, qty)
SELECT i,
       CASE WHEN i % 3 = 0 THEN repeat('kiwi ', 1 + i % 4) ELSE '' END
           || repeat('filler ', i % 13) || 'apple fruit ' || i,
       'item ' || i,
       (i % 100)::FLOAT8,
       i % 7
FROM generate_series(1, 20000) i;

CREATE INDEX hf_topk_idx ON hf_topk USING bm25 (id, body, qty)
    WITH (target_segment_count = 4);
CREATE INDEX hf_topk_price ON hf_topk (price);
VACUUM ANALYZE hf_topk;

-- Arithmetic across columns: a heap filter under the Top K.
EXPLAIN (COSTS OFF)
SELECT id, pdb.score(id) FROM hf_topk
WHERE body ||| 'kiwi' AND price * qty > 300
ORDER BY pdb.score(id) DESC, id LIMIT 10;

SELECT id, pdb.score(id) FROM hf_topk
WHERE body ||| 'kiwi' AND price * qty > 300
ORDER BY pdb.score(id) DESC, id LIMIT 10;

-- Rows that differ from the reference, in either direction.
CREATE FUNCTION hf_topk_diff(indexed TEXT, heap TEXT, k INT) RETURNS BIGINT
LANGUAGE plpgsql AS $$
DECLARE
    topk TEXT := format(
        'SELECT id, pdb.score(id) AS s FROM hf_topk WHERE %s AND %s
         ORDER BY pdb.score(id) DESC, id LIMIT %s', indexed, heap, k);
    reference TEXT := format(
        'SELECT id, s FROM (
             SELECT id, pdb.score(id) AS s, title, price, qty FROM hf_topk
             WHERE %s OFFSET 0) everything
         WHERE %s ORDER BY s DESC, id LIMIT %s', indexed, heap, k);
    n BIGINT;
BEGIN
    EXECUTE format('SELECT count(*) FROM (((%s) EXCEPT ALL (%s)) UNION ALL ((%s) EXCEPT ALL (%s))) d',
                   topk, reference, reference, topk) INTO n;
    RETURN n;
END $$;

SELECT indexed, heap, k, hf_topk_diff(indexed, heap, k) AS rows_differ
FROM (VALUES
    ($$body ||| 'kiwi'$$, 'price * qty > 300', 10),
    ($$body ||| 'kiwi'$$, 'id % 4 <> 3', 10),
    ($$body ||| 'kiwi'$$, 'length(title) > 9', 10),
    ($$body ||| 'kiwi'$$, $$title LIKE 'item 1%'$$, 25),
    ($$body ||| 'kiwi'$$, 'price * qty > 300 AND length(title) > 9', 10),
    ($$body ||| 'kiwi'$$, 'price < 3', 10),
    ($$body ||| 'kiwi'$$, 'price * qty > 300', 5000),
    ($$body ||| 'kiwi'$$, 'price < 0', 10),
    ($$body ||| 'kiwi' AND qty <> 6$$, 'price * qty > 300', 10),
    ($$body ||| 'kiwi apple'$$, 'id % 4 <> 3', 10),
    ($$body ||| 'apple'$$, 'price * qty > 300', 10)
) v(indexed, heap, k);

-- Two heap filters with an indexed range.
EXPLAIN (COSTS OFF)
SELECT id FROM hf_topk
WHERE body ||| 'kiwi' AND qty <> 6 AND price * qty > 300 AND length(title) > 9
ORDER BY pdb.score(id) DESC LIMIT 10;

-- A filter no row passes.
SELECT id FROM hf_topk
WHERE body ||| 'kiwi' AND price < 0
ORDER BY pdb.score(id) DESC LIMIT 10;

-- The filter runs on a fraction of the 6666 "kiwi" matches.
CREATE SEQUENCE hf_topk_checks;
CREATE FUNCTION hf_topk_expensive(p FLOAT8, q INT) RETURNS BOOL STABLE LANGUAGE plpgsql AS $$
BEGIN
    PERFORM nextval('hf_topk_checks');
    RETURN p * q > 300;
END $$;

SELECT id FROM hf_topk
WHERE body ||| 'kiwi' AND hf_topk_expensive(price, qty)
ORDER BY pdb.score(id) DESC, id LIMIT 10;
SELECT nextval('hf_topk_checks') < 1000 AS checked_few_rows;

-- The bitmap of the btree index on price under a score-ordered Top K.
EXPLAIN (COSTS OFF)
SELECT id FROM hf_topk
WHERE body ||| 'kiwi' AND price = 42
ORDER BY pdb.score(id) DESC, id LIMIT 10;
SELECT hf_topk_diff($$body ||| 'kiwi'$$, 'price = 42', 10) AS rows_differ;

DROP TABLE hf_topk CASCADE;
DROP FUNCTION hf_topk_diff(TEXT, TEXT, INT);
DROP FUNCTION hf_topk_expensive(FLOAT8, INT);
DROP SEQUENCE hf_topk_checks;

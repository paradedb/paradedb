\i common/common_setup.sql

-- Cancel inside an all-rejected heap filter without relying on query duration.
CREATE TABLE heap_filter_cancel (id integer, body text, extra integer);
INSERT INTO heap_filter_cancel SELECT g, 'fox', 0 FROM generate_series(1, 32) g;
CREATE INDEX heap_filter_cancel_idx ON heap_filter_cancel USING paradedb (id, body)
WITH (target_segment_count = 1);

-- Prime the sequence cache so counting candidates does not require disk access.
CREATE SEQUENCE heap_filter_cancel_calls CACHE 100;
SELECT nextval('heap_filter_cancel_calls');

EXPLAIN (COSTS OFF)
SELECT id FROM heap_filter_cancel
WHERE body ||| 'fox' AND (
    pg_cancel_backend(pg_backend_pid() + (id - id)
        + 0 * nextval('heap_filter_cancel_calls')::integer) IS FALSE
    OR extra = -1
)
LIMIT 1;

SELECT id FROM heap_filter_cancel
WHERE body ||| 'fox' AND (
    pg_cancel_backend(pg_backend_pid() + (id - id)
        + 0 * nextval('heap_filter_cancel_calls')::integer) IS FALSE
    OR extra = -1
)
LIMIT 1;

-- The cancel must be processed before evaluating a second candidate.
SELECT currval('heap_filter_cancel_calls') - 1 AS candidates_evaluated;

SELECT count(*) FROM heap_filter_cancel WHERE body ||| 'fox' AND extra = 0;

-- A cancel raised by the last rejected candidate must not be lost at EOF.
TRUNCATE heap_filter_cancel;
INSERT INTO heap_filter_cancel VALUES (1, 'fox', 0);
SELECT id FROM heap_filter_cancel
WHERE body ||| 'fox' AND (
    pg_cancel_backend(pg_backend_pid() + (id - id)
        + 0 * nextval('heap_filter_cancel_calls')::integer) IS FALSE
    OR extra = -1
)
LIMIT 1;

DROP TABLE heap_filter_cancel;
DROP SEQUENCE heap_filter_cancel_calls;

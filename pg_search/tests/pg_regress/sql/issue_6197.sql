-- Aggregates over JSON subpaths should use AggregateScan, not BaseScan columnar pullup.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan = on;
SET paradedb.enable_columnar_exec = on;
SET max_parallel_workers_per_gather = 0;

CREATE TABLE issue_6197_json_fast_field_repro (
    id bigint PRIMARY KEY,
    body text NOT NULL,
    custom jsonb NOT NULL
);

INSERT INTO issue_6197_json_fast_field_repro VALUES
    (1, 'common', '{"score": 991}'),
    (2, 'term',   '{"score": 992}'),
    (3, 'common', '{"score": 500}'),
    (4, 'term',   '{"score": 999}'),
    (5, 'common', '{"score": 1}');

CREATE INDEX issue_6197_json_fast_field_repro_idx
ON issue_6197_json_fast_field_repro USING paradedb (
    id,
    body,
    (custom::pdb.simple('columnar=true'))
) WITH (key_field = 'id');

VACUUM (ANALYZE) issue_6197_json_fast_field_repro;

EXPLAIN (VERBOSE, COSTS OFF)
SELECT count(*), min(s), max(s)
FROM (
    SELECT (custom->>'score')::bigint AS s
    FROM issue_6197_json_fast_field_repro
    WHERE id @@@ paradedb.parse('custom.score:>990')
) q;

SELECT count(*), min(s), max(s)
FROM (
    SELECT (custom->>'score')::bigint AS s
    FROM issue_6197_json_fast_field_repro
    WHERE id @@@ paradedb.parse('custom.score:>990')
) q;

SET paradedb.enable_aggregate_custom_scan = off;

EXPLAIN (VERBOSE, COSTS OFF)
SELECT count(*), min(s), max(s)
FROM (
    SELECT (custom->>'score')::bigint AS s
    FROM issue_6197_json_fast_field_repro
    WHERE id @@@ paradedb.parse('custom.score:>990')
) q;

SELECT count(*), min(s), max(s)
FROM (
    SELECT (custom->>'score')::bigint AS s
    FROM issue_6197_json_fast_field_repro
    WHERE id @@@ paradedb.parse('custom.score:>990')
) q;

DROP TABLE issue_6197_json_fast_field_repro;

RESET max_parallel_workers_per_gather;
RESET paradedb.enable_columnar_exec;
RESET paradedb.enable_aggregate_custom_scan;

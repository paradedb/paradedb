\i common/common_setup.sql

-- A Sort above Normal/Columnar scans consumes every match, regardless of LIMIT.
SET paradedb.global_mutable_segment_rows = 0;
SET paradedb.min_rows_per_worker = 0;
SET paradedb.planner_warnings = 'off';
SET jit = off;

CREATE TABLE limit_cost (id INTEGER PRIMARY KEY, body TEXT, label TEXT COLLATE "en-x-icu");
CREATE INDEX limit_cost_idx ON limit_cost
USING paradedb (id, (body::pdb.simple('columnar=true')), (label::pdb.literal))
WITH (layer_sizes = '0', background_layer_sizes = '0');
INSERT INTO limit_cost SELECT i, 'common alpha', md5(i::text) FROM generate_series(1, 1000) i;
INSERT INTO limit_cost SELECT i, 'common beta', md5(i::text) FROM generate_series(1001, 2000) i;
INSERT INTO limit_cost SELECT i, 'common alpha', md5(i::text) FROM generate_series(2001, 3000) i;
INSERT INTO limit_cost SELECT i, 'common beta', md5(i::text) FROM generate_series(3001, 4000) i;
ANALYZE limit_cost;

CREATE FUNCTION pg_temp.limit_cost_scan(query TEXT) RETURNS JSONB
LANGUAGE plpgsql AS $$
DECLARE
    plan JSONB;
BEGIN
    EXECUTE 'EXPLAIN (FORMAT JSON) ' || query INTO plan;
    RETURN jsonb_path_query_first(plan,
        '$.** ? (@."Custom Plan Provider" == "ParadeDB Base Scan")');
END
$$;

-- The intrinsic scan cardinality and cost must be independent of a LIMIT above
-- an unsupported ORDER BY. Check both columnar and heap-fetch execution.
SELECT pg_temp.limit_cost_scan($q$
    SELECT id, label FROM limit_cost WHERE body ||| 'common' ORDER BY label LIMIT 10
$q$) AS plan INTO TEMP TABLE limited_scan;
SELECT pg_temp.limit_cost_scan($q$
    SELECT id, label FROM limit_cost WHERE body ||| 'common' ORDER BY label
$q$) AS plan INTO TEMP TABLE full_scan;

SELECT l.plan->>'Exec Method' AS method,
       (l.plan->>'Plan Rows')::int > 10 AS scans_all_matches,
       l.plan->>'Plan Rows' = f.plan->>'Plan Rows' AS same_rows,
       l.plan->>'Total Cost' = f.plan->>'Total Cost' AS same_cost
FROM limited_scan l CROSS JOIN full_scan f;

SET paradedb.enable_columnar_exec = off;
TRUNCATE limited_scan, full_scan;
INSERT INTO limited_scan SELECT pg_temp.limit_cost_scan($q$
    SELECT id FROM limit_cost WHERE body ||| 'common' ORDER BY sqrt(id::float8) LIMIT 10
$q$);
INSERT INTO full_scan SELECT pg_temp.limit_cost_scan($q$
    SELECT id FROM limit_cost WHERE body ||| 'common' ORDER BY sqrt(id::float8)
$q$);
SELECT l.plan->>'Exec Method' AS method,
       (l.plan->>'Plan Rows')::int > 10 AS scans_all_matches,
       l.plan->>'Plan Rows' = f.plan->>'Plan Rows' AS same_rows,
       l.plan->>'Total Cost' = f.plan->>'Total Cost' AS same_cost
FROM limited_scan l CROSS JOIN full_scan f;

RESET paradedb.enable_columnar_exec;

-- TopK really bounds output, including OFFSET, and must retain its estimates.
SELECT plan->>'Exec Method' AS method, (plan->>'Plan Rows')::int AS rows
FROM (SELECT pg_temp.limit_cost_scan($q$
    SELECT id FROM limit_cost WHERE body ||| 'common' ORDER BY id LIMIT 10 OFFSET 5
$q$) AS plan) p;

-- Keep generic plans from applying their estimated LIMIT to a computed sort.
SET plan_cache_mode = force_generic_plan;
PREPARE limit_cost_computed(int) AS
    SELECT id FROM limit_cost WHERE body ||| 'common' ORDER BY sqrt(id::float8) LIMIT $1;
SELECT (plan->>'Plan Rows')::int > 1000 AS scans_past_parameter_estimate
FROM (SELECT pg_temp.limit_cost_scan('EXECUTE limit_cost_computed(10)') AS plan) p;
PREPARE limit_cost_collation(int, int) AS
    SELECT id, label FROM limit_cost WHERE body ||| 'common' ORDER BY label LIMIT $1 OFFSET $2;
SELECT plan->>'Exec Method' AS method,
       (plan->>'Plan Rows')::int > 1000 AS scans_past_parameter_estimate
FROM (SELECT pg_temp.limit_cost_scan('EXECUTE limit_cost_collation(10, 5)') AS plan) p;

-- A runtime predicate exercises the uncostable worker-heuristic path. The
-- computed sort still needs every segment, even though the final LIMIT is 10.
SET max_parallel_workers_per_gather = 2;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
PREPARE limit_cost_runtime(text) AS
    SELECT id FROM limit_cost WHERE body ||| $1 AND id > 0
    ORDER BY sqrt(id::float8) LIMIT 10;
SELECT (plan->>'Parallel Aware')::boolean AS parallel_scan,
       (plan->>'Plan Rows')::int > 10 AS scans_all_matches
FROM (SELECT pg_temp.limit_cost_scan('EXECUTE limit_cost_runtime(''common'')') AS plan) p;

-- Also exercise the costable path with a fixed predicate and several segments.
SELECT (plan->>'Parallel Aware')::boolean AS parallel_scan,
       (plan->>'Plan Rows')::int > 10 AS scans_all_matches
FROM (SELECT pg_temp.limit_cost_scan($q$
    SELECT id FROM limit_cost WHERE body ||| 'common' ORDER BY sqrt(id::float8) LIMIT 10
$q$) AS plan) p;

SELECT plan->>'Exec Method' AS method,
       (plan->>'Parallel Aware')::boolean AS parallel_scan,
       (plan->>'Plan Rows')::int > 10 AS scans_all_matches
FROM (SELECT pg_temp.limit_cost_scan($q$
    SELECT id, label FROM limit_cost WHERE body ||| 'common' ORDER BY label LIMIT 10
$q$) AS plan) p;

-- Verify the full sort sees late-arriving winners, including OFFSET. Compare
-- the fixed and runtime-bound forms to ordinary PostgreSQL execution.
SET max_parallel_workers_per_gather = 0;
WITH actual AS (
    SELECT id FROM limit_cost WHERE body ||| 'common'
    ORDER BY -sqrt(id::float8) LIMIT 10 OFFSET 5
), expected AS (
    SELECT id FROM limit_cost WHERE body LIKE 'common%'
    ORDER BY -sqrt(id::float8) LIMIT 10 OFFSET 5
)
SELECT (SELECT array_agg(id ORDER BY id) FROM actual) =
       (SELECT array_agg(id ORDER BY id) FROM expected) AS matches_postgres;
WITH actual AS (
    SELECT id FROM limit_cost WHERE body ||| 'common' ORDER BY label LIMIT 10 OFFSET 5
), expected AS (
    SELECT id FROM limit_cost WHERE body LIKE 'common%' ORDER BY label LIMIT 10 OFFSET 5
)
SELECT (SELECT array_agg(id ORDER BY id) FROM actual) =
       (SELECT array_agg(id ORDER BY id) FROM expected) AS matches_postgres;
EXECUTE limit_cost_computed(10);

-- A generic Columnar plan resolves its batch hint at execution time. Small
-- batches must still deliver all matches to the external sort, including when
-- the same plan is executed again with different LIMIT/OFFSET values.
PREPARE limit_cost_batch_results(bigint, bigint) AS
WITH actual AS (
    SELECT id, label FROM limit_cost WHERE body ||| 'common'
    ORDER BY label LIMIT $1 OFFSET $2
), expected AS (
    SELECT id, label FROM limit_cost WHERE body LIKE 'common%'
    ORDER BY label LIMIT $1 OFFSET $2
)
SELECT (SELECT array_agg(ROW(id, label)::text ORDER BY id) FROM actual)
       IS NOT DISTINCT FROM
       (SELECT array_agg(ROW(id, label)::text ORDER BY id) FROM expected)
       AS prepared_matches_postgres;
SELECT plan->>'Exec Method' AS method
FROM (SELECT pg_temp.limit_cost_scan('EXECUTE limit_cost_batch_results(10, 5)') AS plan) p;
EXECUTE limit_cost_batch_results(10, 5);
EXECUTE limit_cost_batch_results(3, 1);
EXECUTE limit_cost_batch_results(0, 0);
EXECUTE limit_cost_batch_results(NULL, NULL);

-- Doubling the resolved fetch must saturate before the scanner clamps its hint.
-- In particular, max bigint plus OFFSET 1 must not wrap the hint to zero.
SET statement_timeout = '30s';
EXECUTE limit_cost_batch_results(9223372036854775807, 1);
EXECUTE limit_cost_batch_results(9223372036854775807, 9223372036854775807);
RESET statement_timeout;
DEALLOCATE limit_cost_batch_results;

DEALLOCATE limit_cost_computed;
DEALLOCATE limit_cost_runtime;
DEALLOCATE limit_cost_collation;
DROP TABLE limited_scan, full_scan, limit_cost;
RESET plan_cache_mode;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET max_parallel_workers_per_gather;
RESET paradedb.global_mutable_segment_rows;
RESET paradedb.min_rows_per_worker;
RESET paradedb.planner_warnings;
RESET jit;
\i common/common_cleanup.sql

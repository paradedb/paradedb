-- Regression test derived from qgen::generated_group_by_aggregates.
--
-- Bug class:
--   ParadeDB pushdown predicate + PostgreSQL residual qual + TopK/LIMIT-like
--   execution must not apply TopK before the residual qual.
--
-- Critical requirement:
--   rating is intentionally NOT part of the BM25 index. Therefore rating = 4
--   must remain a PostgreSQL residual Filter, not a Tantivy/ParadeDB query.
--
-- Broken behavior:
--   TopKScanExecState picks id = 1 / rating = 5 from the ParadeDB-side query,
--   then PostgreSQL residual Filter rating = 4 removes it, leaving an empty
--   input and producing no row / NULL.
--
-- Correct behavior:
--   residual Filter is applied before LIMIT/MAX semantics can discard
--   candidates, so id = 2 / rating = 4 survives.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;

SET paradedb.enable_custom_scan = on;
SET paradedb.enable_filter_pushdown = off;
SET paradedb.enable_columnar_exec = off;
SET paradedb.enable_aggregate_custom_scan = off;
SET paradedb.enable_join_custom_scan = off;
SET paradedb.enable_custom_scan_without_operator = off;

SET enable_seqscan = off;
SET enable_indexscan = off;

DROP TABLE IF EXISTS issue_4710_topk_residual;

CREATE TABLE issue_4710_topk_residual (
                                          id bigint PRIMARY KEY,
                                          rating integer NOT NULL,
                                          description text
);

INSERT INTO issue_4710_topk_residual (id, rating, description)
VALUES
    (1, 5, 'higher rated row rejected by residual filter'),
    (2, 4, 'correct row that survives residual filter'),
    (4, 4, 'row excluded by ParadeDB id predicate');

-- IMPORTANT:
-- rating is deliberately NOT indexed by bm25.
-- This is what makes rating = 4 a PostgreSQL residual Filter.
CREATE INDEX issue_4710_topk_residual_idx
    ON issue_4710_topk_residual
    USING bm25 (id, description)
    WITH (key_field = 'id');

ANALYZE issue_4710_topk_residual;

CREATE OR REPLACE FUNCTION pg_temp.issue_4710_plan_text(query text)
RETURNS text
LANGUAGE plpgsql
AS $$
DECLARE
line text;
    result text := '';
BEGIN
FOR line IN EXECUTE format(
        'EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) %s',
        query
    )
    LOOP
        result := result || line || E'\n';
END LOOP;

RETURN result;
END
$$;

-- Direct LIMIT form.
-- This makes the TopK-before-residual problem explicit.
WITH plan AS (
    SELECT pg_temp.issue_4710_plan_text($q$
        SELECT rating
        FROM issue_4710_topk_residual
        WHERE rating = 4
          AND NOT (id @@@ '4')
        ORDER BY rating DESC
        LIMIT 1
    $q$) AS p
)
SELECT
    p LIKE '%Custom Scan (ParadeDB Base Scan)%' AS uses_paradedb_custom_scan,
    p LIKE '%Filter:%rating = 4%' AS has_residual_rating_filter,
    p NOT LIKE '%Exec Method: TopKScanExecState%' AS does_not_use_topk_with_residual,
    p LIKE '%Exec Method: NormalScanExecState%' AS uses_normal_scan,
    p LIKE '%must_not%' AS has_paradedb_must_not_query
FROM plan;

SELECT rating AS postgres_top_rating
FROM issue_4710_topk_residual
WHERE rating = 4
  AND NOT (id = 4)
ORDER BY rating DESC
    LIMIT 1;

SELECT rating AS paradedb_top_rating
FROM issue_4710_topk_residual
WHERE rating = 4
  AND NOT (id @@@ '4')
ORDER BY rating DESC
    LIMIT 1;

-- Aggregate form, closer to qgen::generated_group_by_aggregates.
WITH plan AS (
    SELECT pg_temp.issue_4710_plan_text($q$
        SELECT MAX(rating)
        FROM issue_4710_topk_residual
        WHERE rating = 4
          AND NOT (id @@@ '4')
          AND rating = 4
    $q$) AS p
)
SELECT
    p LIKE '%Custom Scan (ParadeDB Base Scan)%' AS uses_paradedb_custom_scan,
    p LIKE '%Filter:%rating = 4%' AS has_residual_rating_filter,
    p NOT LIKE '%Exec Method: TopKScanExecState%' AS does_not_use_topk_with_residual,
    p LIKE '%Exec Method: NormalScanExecState%' AS uses_normal_scan,
    p LIKE '%must_not%' AS has_paradedb_must_not_query
FROM plan;

SELECT MAX(rating) AS postgres_max_rating
FROM issue_4710_topk_residual
WHERE rating = 4
  AND NOT (id = 4)
  AND rating = 4;

SELECT MAX(rating) AS paradedb_max_rating
FROM issue_4710_topk_residual
WHERE rating = 4
  AND NOT (id @@@ '4')
  AND rating = 4;

DROP TABLE issue_4710_topk_residual;

RESET enable_indexscan;
RESET enable_seqscan;

RESET paradedb.enable_custom_scan_without_operator;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_columnar_exec;
RESET paradedb.enable_filter_pushdown;
RESET paradedb.enable_custom_scan;

RESET max_parallel_workers_per_gather;
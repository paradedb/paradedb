-- Regression tests for:
--   https://github.com/paradedb/paradedb/issues/4710
--
-- Related earlier issue:
--   https://github.com/paradedb/paradedb/issues/4596
--
-- Core invariant:
--   If pdb.score(...) is required and a mandatory ParadeDB search predicate
--   exists in top-level AND context, PostgreSQL-evaluable residual predicates
--   must not force fallback to a plain Index Scan / Index Only Scan.
--
-- These tests intentionally do not compare full EXPLAIN output. Instead, each
-- case returns stable boolean planner/executor invariants.

SET client_min_messages TO warning;
CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;

DROP TABLE IF EXISTS mock_items CASCADE;

CALL paradedb.create_bm25_test_table(
    schema_name => 'public',
    table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');

ANALYZE mock_items;

CREATE TEMP TABLE issue_4710_config (
    name text PRIMARY KEY,
    enabled boolean NOT NULL
);

INSERT INTO issue_4710_config(name, enabled)
VALUES
    ('enabled', true),
    ('disabled', false);

CREATE FUNCTION pg_temp.issue_4710_has_access()
    RETURNS boolean
    LANGUAGE sql
    STABLE
AS $$
SELECT true
           $$;

CREATE FUNCTION pg_temp.issue_4710_threshold()
    RETURNS integer
    LANGUAGE sql
    STABLE
AS $$
SELECT 100
           $$;

CREATE FUNCTION pg_temp.check_scored_custom_scan(
    p_case_no text,
    p_case_name text,
    p_query text
)
    RETURNS TABLE (
                      case_no text,
                      case_name text,
                      explain_ok boolean,
                      execution_ok boolean,
                      custom_scan boolean,
                      scores_true boolean,
                      tantivy_query boolean,
                      no_plain_index_scan boolean,
                      score_evaluated boolean,
                      passed boolean,
                      error_kind text
                  )
    LANGUAGE plpgsql
AS $$
DECLARE
plan_line text;
    saw_plain_search_index_scan boolean := false;
    scored_rows bigint := 0;
BEGIN
    case_no := p_case_no;
    case_name := p_case_name;

    explain_ok := false;
    execution_ok := false;
    custom_scan := false;
    scores_true := false;
    tantivy_query := false;
    no_plain_index_scan := true;
    score_evaluated := false;
    passed := false;
    error_kind := '';

BEGIN
FOR plan_line IN EXECUTE
            format('EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) %s', p_query)
        LOOP
            custom_scan :=
                custom_scan
                OR plan_line LIKE '%Custom Scan (ParadeDB Base Scan)%';

            scores_true :=
                scores_true
                OR plan_line LIKE '%Scores: true%';

            tantivy_query :=
                tantivy_query
                OR plan_line LIKE '%Tantivy Query:%';

            saw_plain_search_index_scan :=
                saw_plain_search_index_scan
                OR plan_line LIKE '%Index Scan using search_idx%'
                OR plan_line LIKE '%Index Only Scan using search_idx%';
END LOOP;

        explain_ok := true;
        no_plain_index_scan := NOT saw_plain_search_index_scan;
EXCEPTION WHEN OTHERS THEN
        explain_ok := false;
        error_kind :=
            CASE
                WHEN SQLERRM LIKE 'Unsupported query shape%' THEN 'unsupported_query_shape'
                ELSE SQLSTATE
END;
END;

BEGIN
        -- Force actual evaluation of the score output column. A plain count(*)
        -- over the subquery would not be enough, because PostgreSQL could avoid
        -- evaluating an unused targetlist expression.
EXECUTE format(
        'SELECT count(score)::bigint FROM (%s) AS scored_query WHERE score IS NOT NULL',
        p_query
        )
    INTO scored_rows;

execution_ok := true;
        score_evaluated := scored_rows > 0;
EXCEPTION WHEN OTHERS THEN
        execution_ok := false;
        score_evaluated := false;
        error_kind :=
            CASE
                WHEN SQLERRM LIKE 'Unsupported query shape%' THEN 'unsupported_query_shape'
                ELSE SQLSTATE
END;
END;

    passed :=
        explain_ok
        AND execution_ok
        AND custom_scan
        AND scores_true
        AND tantivy_query
        AND no_plain_index_scan
        AND score_evaluated;

RETURN NEXT;
END
$$;

CREATE TEMP TABLE issue_4710_results AS
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '01',
        'issue_4710_original',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes' AND ((SELECT true) OR id < 4)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '02',
        'issue_4596_top_level_initplan_and',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes' AND (SELECT true)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '03',
        'literal_or_residual',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes' AND (true OR id < 4)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '04',
        'reversed_initplan_or',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes' AND (id < 4 OR (SELECT true))
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '05',
        'false_initplan_inside_or',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes' AND ((SELECT false) OR id < 100)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '06',
        'nested_or_and_residual',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND ((SELECT true) OR (id < 4 AND length(description) > 0))
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '07',
        'table_backed_initplan_inside_or',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND (
              (
                  SELECT enabled
                  FROM issue_4710_config
                  WHERE name = 'enabled'
                  LIMIT 1
              )
              OR id < 4
          )
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '08',
        'stable_function_subquery_inside_or',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND ((SELECT pg_temp.issue_4710_has_access()) OR id < 4)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '09',
        'exists_inside_or',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND (EXISTS (SELECT 1) OR id < 4)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '10',
        'correlated_exists_inside_or',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND (
              EXISTS (
                  SELECT 1
                  WHERE mock_items.id > 0
              )
              OR id < 4
          )
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '11',
        'scalar_initplan_inside_opexpr',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND id < (SELECT pg_temp.issue_4710_threshold())
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '12',
        'case_expression_with_initplan',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND CASE
                  WHEN (SELECT true) THEN id < 100
                  ELSE id < 0
              END
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '13',
        'coalesce_expression_with_initplan',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND COALESCE((SELECT true), id < 4)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '14',
        'not_and_expression_with_initplan',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND NOT ((SELECT false) AND id > 4)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '15',
        'scalar_array_opexpr_inside_or',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND ((SELECT true) OR id = ANY (ARRAY[1, 2, 3]))
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '16',
        'score_nested_inside_target_expression',
        $q$
            SELECT id, pdb.score(id) + 0.0::real AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND ((SELECT true) OR id < 4)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '17',
        'score_used_in_order_by',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND ((SELECT true) OR id < 4)
        ORDER BY pdb.score(id) DESC
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '18',
        'cte_backed_initplan_inside_or',
        $q$
            WITH enabled AS MATERIALIZED (
            SELECT true AS value
        )
        SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND ((SELECT value FROM enabled) OR id < 4)
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '19',
        'multiple_residual_conjuncts',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND ((SELECT true) OR id < 4)
          AND CASE
                  WHEN (SELECT true) THEN length(description) > 0
                  ELSE false
              END
        LIMIT 5
    $q$
     );

INSERT INTO issue_4710_results
SELECT *
FROM pg_temp.check_scored_custom_scan(
        '20',
        'non_subquery_residual_expression',
        $q$
            SELECT id, pdb.score(id) AS score
        FROM mock_items
        WHERE description ||| 'shoes'
          AND length(description) > 0
        LIMIT 5
    $q$
     );

SELECT
    case_no,
    case_name,
    explain_ok,
    execution_ok,
    custom_scan,
    scores_true,
    tantivy_query,
    no_plain_index_scan,
    score_evaluated,
    passed,
    error_kind
FROM issue_4710_results
ORDER BY case_no;

DROP TABLE issue_4710_results;
DROP TABLE issue_4710_config;
DROP TABLE mock_items CASCADE;

RESET max_parallel_workers_per_gather;
RESET client_min_messages;
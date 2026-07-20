CREATE TABLE issue_5546_generic_residual_items
(
    id       bigint PRIMARY KEY,
    body     text    NOT NULL,
    allowed  boolean,
    priority integer NOT NULL
);

INSERT INTO issue_5546_generic_residual_items (id, body, allowed, priority)
VALUES (1, 'alpha alpha alpha', false, 100),
       (2, 'alpha alpha', true, 90),
       (3, 'alpha', NULL, 80),
       (4, 'beta', true, 70);

CREATE INDEX issue_5546_generic_residual_items_idx
    ON issue_5546_generic_residual_items
    USING bm25 (id, body, priority)
    WITH (
        key_field = 'id',
        numeric_fields = '{"priority": {"fast": true}}'
    );

ANALYZE issue_5546_generic_residual_items;

SET max_parallel_workers_per_gather = 0;
SET enable_seqscan = off;
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;

-- Keep negative plan assertions stable without depending on the deparsed
-- fallback Seq Scan expression (which can contain installation-specific OIDs).
CREATE OR REPLACE FUNCTION pg_temp.issue_5546_plan_uses_basescan(query text)
RETURNS boolean
LANGUAGE plpgsql
AS $$
DECLARE
    line record;
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) ' || query LOOP
        IF line."QUERY PLAN" LIKE '%Custom Scan (ParadeDB Base Scan)%' THEN
            RETURN true;
        END IF;
    END LOOP;
    RETURN false;
END;
$$;

-- Reproduction 1: DistinctExpr is evaluated by PostgreSQL as a residual qual.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id) AS score
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND allowed IS DISTINCT FROM false
ORDER BY id;

SELECT id, pdb.score(id) IS NOT NULL AS score_available
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND allowed IS DISTINCT FROM false
ORDER BY id;

-- Reproduction 2: CaseExpr is evaluated by PostgreSQL as a residual qual.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id) AS score
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND CASE
          WHEN priority >= 90 THEN allowed
          ELSE id = 3
      END
ORDER BY id;

SELECT id, pdb.score(id) IS NOT NULL AS score_available
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND CASE
          WHEN priority >= 90 THEN allowed
          ELSE id = 3
      END
ORDER BY id;

-- Reproduction 3: a nested InitPlan/PARAM_EXEC remains inside the complete
-- CoalesceExpr evaluated by PostgreSQL.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id) AS score
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND COALESCE((SELECT NULL::boolean), allowed, false)
ORDER BY id;

SELECT id, pdb.score(id) IS NOT NULL AS score_available
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND COALESCE((SELECT NULL::boolean), allowed, false)
ORDER BY id;

-- Reproduction 4: LIMIT must observe rows after PostgreSQL residual filtering.
-- id=1 has the greatest priority but fails the residual; id=2 is correct.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, priority, pdb.score(id) AS score
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND allowed IS DISTINCT FROM false
ORDER BY priority DESC
LIMIT 1;

SELECT id, priority, pdb.score(id) IS NOT NULL AS score_available
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND allowed IS DISTINCT FROM false
ORDER BY priority DESC
LIMIT 1;

-- An unsupported wrapper must not hide a nested ParadeDB search operator.
SELECT NOT pg_temp.issue_5546_plan_uses_basescan(
    $$SELECT id, pdb.score(id)
        FROM issue_5546_generic_residual_items
       WHERE body ||| 'alpha'
         AND CASE WHEN allowed IS false THEN body ||| 'beta' ELSE true END$$
) AS nested_search_residual_rejected;

SELECT id, pdb.score(id) AS score
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND CASE
          WHEN allowed IS false THEN body ||| 'beta'
          ELSE true
      END;

-- A real correlated SubPlan must not bypass that fail-closed check.
SELECT NOT pg_temp.issue_5546_plan_uses_basescan(
    $$SELECT outer_items.id, pdb.score(outer_items.id)
        FROM issue_5546_generic_residual_items AS outer_items
       WHERE outer_items.body ||| 'alpha'
         AND CASE
                 WHEN EXISTS (
                     SELECT 1
                     FROM issue_5546_generic_residual_items AS inner_items
                     WHERE inner_items.id = outer_items.id
                       AND inner_items.allowed IS false
                 )
                 THEN outer_items.body ||| 'beta'
                 ELSE true
             END$$
) AS correlated_nested_search_rejected;

SELECT outer_items.id, pdb.score(outer_items.id) AS score
FROM issue_5546_generic_residual_items AS outer_items
WHERE outer_items.body ||| 'alpha'
  AND CASE
          WHEN EXISTS (
              SELECT 1
              FROM issue_5546_generic_residual_items AS inner_items
              WHERE inner_items.id = outer_items.id
                AND inner_items.allowed IS false
          )
          THEN outer_items.body ||| 'beta'
          ELSE true
      END;

-- Context-dependent score/snippet placeholders cannot be raw ExecQual residuals.
SELECT NOT pg_temp.issue_5546_plan_uses_basescan(
    $$SELECT id, pdb.score(id)
        FROM issue_5546_generic_residual_items
       WHERE body ||| 'alpha'
         AND CASE WHEN allowed THEN pdb.score(id) > 0 ELSE true END$$
) AS score_residual_rejected;

SELECT id, pdb.score(id) AS score
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND CASE
          WHEN allowed THEN pdb.score(id) > 0
          ELSE true
      END;

-- Internal aggregate placeholders are ordinary FuncExpr nodes, so PostgreSQL
-- accepts them syntactically in WHERE even though only ParadeDB may populate
-- them. They must fail closed as residuals as well.
-- A raw call to one of the panic-only search implementation functions can
-- remain a FuncExpr when planner support cannot associate its lhs with the
-- indexed relation. It must not become a residual either.
SELECT
    NOT pg_temp.issue_5546_plan_uses_basescan(
        $$SELECT id, pdb.score(id)
            FROM issue_5546_generic_residual_items
           WHERE body ||| 'alpha'
             AND pdb.agg_fn('count') IS NOT NULL$$
    ) AS aggregate_placeholder_residual_rejected,
    NOT pg_temp.issue_5546_plan_uses_basescan(
        $$SELECT id, pdb.score(id)
            FROM issue_5546_generic_residual_items
           WHERE body ||| 'alpha'
             AND pdb.window_agg('{}') > 0$$
    ) AS window_placeholder_residual_rejected,
    NOT pg_temp.issue_5546_plan_uses_basescan(
        $$SELECT id, pdb.score(id)
            FROM issue_5546_generic_residual_items
           WHERE body ||| 'alpha'
             AND CASE
                     WHEN allowed THEN paradedb.search_with_parse(
                         'not-a-column'::text,
                         clock_timestamp()::text
                     )
                     ELSE true
                 END$$
    ) AS raw_search_function_residual_rejected;

-- ParadeDB value builders are ordinary PostgreSQL-executable expressions.
-- Inside an otherwise unsupported top-level conjunct they remain valid
-- residuals; only a boolean search predicate establishes search context.
SELECT pg_temp.issue_5546_plan_uses_basescan(
    $$SELECT id, pdb.score(id)
        FROM issue_5546_generic_residual_items
       WHERE body ||| 'alpha'
         AND CASE
                 WHEN allowed IS NULL THEN
                     ('left' ## (priority % 2) ## 'right') IS NOT NULL
                     AND pdb.term(priority::bigint) IS NOT NULL
                 ELSE true
             END$$
) AS paradedb_value_residual_allowed;

SELECT id
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND CASE
          WHEN allowed IS NULL THEN
              ('left' ## (priority % 2) ## 'right') IS NOT NULL
              AND pdb.term(priority::bigint) IS NOT NULL
          ELSE true
      END
ORDER BY id;

SET paradedb.enable_filter_pushdown = false;

-- The GUC retains its original opt-out for generic residual classification.
SELECT id, pdb.score(id) IS NOT NULL AS score_available
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND allowed IS DISTINCT FROM false
ORDER BY id;

-- Legacy is_subplan fallback remains available while the GUC is disabled.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.score(id) AS score
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND (SELECT true)
LIMIT 1;

SELECT id, pdb.score(id) IS NOT NULL AS score_available
FROM issue_5546_generic_residual_items
WHERE body ||| 'alpha'
  AND (SELECT true)
ORDER BY id;

RESET paradedb.enable_filter_pushdown;
RESET max_parallel_workers_per_gather;
RESET enable_seqscan;
RESET enable_indexscan;
RESET enable_indexonlyscan;
RESET enable_bitmapscan;

DROP TABLE issue_5546_generic_residual_items;

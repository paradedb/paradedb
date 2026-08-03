CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;

-- #5751 predicate-normalization contract:
--   1. T_List and top-level AND are containers, never executable expressions.
--   2. Each conjunct is classified independently; OR and NOT stay intact.
--   3. Single-relation conjuncts remain owned by their base scans.
--   4. A present unsupported predicate declines AggregateScan; it is never dropped.
--   5. Custom prepared plans with Const substitutions remain eligible.
--   6. Base-filter PARAM_EXTERN uses the existing live ExprContext binding path.
--   7. PARAM_EXTERN in cross-table, HAVING, or FILTER expressions declines.

CREATE TABLE issue_5751_series (
    id bigint PRIMARY KEY,
    state text
);
CREATE TABLE issue_5751_entries (
    id bigint PRIMARY KEY,
    series_id bigint,
    user_id text
);

INSERT INTO issue_5751_series VALUES
    (1, 'active'),
    (2, 'inactive'),
    (3, 'active');
INSERT INTO issue_5751_entries VALUES
    (1, 1, 'u1'),
    (2, 1, 'u2'),
    (3, 2, 'u1'),
    (4, 3, 'u1');

CREATE INDEX issue_5751_series_idx
ON issue_5751_series USING bm25 (id, ((state)::pdb.literal))
WITH (key_field = 'id');
CREATE INDEX issue_5751_entries_idx
ON issue_5751_entries USING bm25 (id, series_id, ((user_id)::pdb.literal))
WITH (key_field = 'id');

-- Keep the plan assertion stable without recording DataFusion's physical plan.
CREATE FUNCTION issue_5751_plan_uses(q text, needle text) RETURNS boolean AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (COSTS OFF) ' || q LOOP
    IF r."QUERY PLAN" LIKE '%' || needle || '%' THEN RETURN true; END IF;
  END LOOP;
  RETURN false;
END $$ LANGUAGE plpgsql;

-- Compare the extension path against PostgreSQL's answer instead of blessing a
-- newly generated literal result. Each dynamic statement is planned after its
-- AggregateScan setting is applied.
CREATE FUNCTION issue_5751_result(q text, use_aggregate_scan boolean)
RETURNS jsonb AS $$
DECLARE
  r record;
  result jsonb := '[]'::jsonb;
BEGIN
  PERFORM set_config(
    'paradedb.enable_aggregate_custom_scan',
    use_aggregate_scan::text,
    true
  );
  FOR r IN EXECUTE q LOOP
    result := result || jsonb_build_array(to_jsonb(r));
  END LOOP;
  RETURN result;
END $$ LANGUAGE plpgsql;

-- PostgreSQL retains the two WHERE conjuncts in an implicit-AND List.  Each
-- item is a base predicate even though the container references both tables.
SELECT issue_5751_plan_uses(
  $$SELECT count(*)
    FROM issue_5751_entries e
    JOIN issue_5751_series s ON s.id = e.series_id
    WHERE s.state = 'active' AND e.user_id = 'u1'$$,
  'ParadeDB Aggregate Scan') AS uses_aggregate_scan;

-- Both filters must remain effective: dropping either one returns three rows.
SELECT count(*) AS filtered_count
FROM issue_5751_entries e
JOIN issue_5751_series s ON s.id = e.series_id
WHERE s.state = 'active' AND e.user_id = 'u1';

SELECT issue_5751_result(
  $$SELECT count(*)
    FROM issue_5751_entries e
    JOIN issue_5751_series s ON s.id = e.series_id
    WHERE s.state = 'active' AND e.user_id = 'u1'$$,
  true
) = issue_5751_result(
  $$SELECT count(*)
    FROM issue_5751_entries e
    JOIN issue_5751_series s ON s.id = e.series_id
    WHERE s.state = 'active' AND e.user_id = 'u1'$$,
  false
) AS matches_postgres;

-- The same semantic query can place the equality in the implicit-join WHERE
-- list.  The equality is a join key; the other two conjuncts remain owned by
-- their base scans.
SELECT issue_5751_plan_uses(
  $$SELECT count(*)
    FROM issue_5751_entries e, issue_5751_series s
    WHERE s.id = e.series_id
      AND s.state = 'active'
      AND e.user_id = 'u1'$$,
  'ParadeDB Aggregate Scan') AS implicit_uses_aggregate_scan;

SELECT count(*) AS implicit_filtered_count
FROM issue_5751_entries e, issue_5751_series s
WHERE s.id = e.series_id
  AND s.state = 'active'
  AND e.user_id = 'u1';

-- Only implicit AND containers are normalized.  OR remains one semantic
-- predicate; flattening it would incorrectly require both states at once.
SELECT count(*) AS preserved_or_count
FROM issue_5751_entries e
JOIN issue_5751_series s ON s.id = e.series_id
WHERE (s.state = 'active' OR s.state = 'inactive')
  AND e.user_id = 'u1';

-- The planner error was independent of row count.
TRUNCATE issue_5751_entries, issue_5751_series;
SELECT count(*) AS empty_count
FROM issue_5751_entries e
JOIN issue_5751_series s ON s.id = e.series_id
WHERE s.state = 'active' AND e.user_id = 'u1';

-- Custom prepared plans replace PARAM_EXTERN with Const nodes.  Preserve that
-- existing supported path and verify that this fix does not merely fall back to
-- PostgreSQL for every prepared statement.
INSERT INTO issue_5751_series VALUES
    (1, 'active'),
    (2, 'inactive'),
    (3, 'active');
INSERT INTO issue_5751_entries VALUES
    (1, 1, 'u1'),
    (2, 1, 'u2'),
    (3, 2, 'u1'),
    (4, 3, 'u1');

PREPARE issue_5751_generic(text, text) AS
SELECT count(*)
FROM issue_5751_entries e
JOIN issue_5751_series s ON s.id = e.series_id
WHERE s.state = $1 AND e.user_id = $2;

SET plan_cache_mode = force_custom_plan;
SELECT issue_5751_plan_uses(
  $$EXECUTE issue_5751_generic('active', 'u1')$$,
  'ParadeDB Aggregate Scan') AS custom_uses_aggregate_scan;
EXECUTE issue_5751_generic('active', 'u1');

-- A generic plan with parameters on both inputs chooses a wrapped lower path.
-- AggregateScan must traverse the transparent wrapper, account for the
-- parameterized-path clauses, and remain selected.
SET plan_cache_mode = force_generic_plan;
SELECT issue_5751_plan_uses(
  $$EXECUTE issue_5751_generic('active', 'u1')$$,
  'ParadeDB Aggregate Scan') AS generic_uses_aggregate_scan;
EXECUTE issue_5751_generic('active', 'u1');

DEALLOCATE issue_5751_generic;

-- Main already supports a single generic base-filter parameter through that
-- runtime path. Keep an explicit compatibility assertion so a future safety
-- guard cannot turn this working AggregateScan into fallback or an error.
PREPARE issue_5751_main_compatible(text) AS
SELECT count(*)
FROM issue_5751_entries e
JOIN issue_5751_series s ON s.id = e.series_id
WHERE s.state = $1;

SELECT issue_5751_plan_uses(
  $$EXECUTE issue_5751_main_compatible('active')$$,
  'ParadeDB Aggregate Scan') AS main_compatible_uses_aggregate_scan;
EXECUTE issue_5751_main_compatible('active');
DEALLOCATE issue_5751_main_compatible;

-- Cross-table DataFusion predicates have no executor-side ParamListInfo
-- binding. Decline this shape rather than raising "no value found for
-- parameter" or evaluating an incomplete predicate.
PREPARE issue_5751_cross_param(bigint) AS
SELECT count(*)
FROM issue_5751_entries e
JOIN issue_5751_series s ON s.id = e.series_id
WHERE s.id + $1 > e.series_id;

SET client_min_messages = error;
SELECT issue_5751_plan_uses(
  $$EXECUTE issue_5751_cross_param(0)$$,
  'ParadeDB Aggregate Scan') AS cross_param_uses_aggregate_scan;
EXECUTE issue_5751_cross_param(0);

SELECT issue_5751_result(
  $$EXECUTE issue_5751_cross_param(0)$$,
  true
) = issue_5751_result(
  $$SELECT count(*)
    FROM issue_5751_entries e
    JOIN issue_5751_series s ON s.id = e.series_id
    WHERE s.id + 0 > e.series_id$$,
  false
) AS cross_param_matches_postgres;

DEALLOCATE issue_5751_cross_param;

-- A present HAVING or aggregate FILTER must not be represented as None merely
-- because its PARAM_EXTERN cannot be translated.  Both shapes decline and
-- PostgreSQL evaluates the original expression.
PREPARE issue_5751_having(bigint) AS
SELECT s.state, count(*)
FROM issue_5751_entries e
JOIN issue_5751_series s ON s.id = e.series_id
GROUP BY s.state
HAVING count(*) > $1
ORDER BY s.state;

SELECT issue_5751_plan_uses(
  $$EXECUTE issue_5751_having(1)$$,
  'ParadeDB Aggregate Scan') AS having_uses_aggregate_scan;
EXECUTE issue_5751_having(1);

PREPARE issue_5751_filter(text) AS
SELECT count(*) FILTER (WHERE e.user_id = $1) AS filtered_aggregate_count
FROM issue_5751_entries e
JOIN issue_5751_series s ON s.id = e.series_id;

SELECT issue_5751_plan_uses(
  $$EXECUTE issue_5751_filter('u1')$$,
  'ParadeDB Aggregate Scan') AS filter_uses_aggregate_scan;
EXECUTE issue_5751_filter('u1');

-- The generic prepared executions above must match independently planned
-- PostgreSQL queries with the same parameter values. This catches a present
-- HAVING or FILTER being mistaken for an absent one.
SELECT issue_5751_result(
  $$EXECUTE issue_5751_having(1)$$,
  true
) = issue_5751_result(
  $$SELECT s.state, count(*)
    FROM issue_5751_entries e
    JOIN issue_5751_series s ON s.id = e.series_id
    GROUP BY s.state
    HAVING count(*) > 1
    ORDER BY s.state$$,
  false
) AS having_matches_postgres;

SELECT issue_5751_result(
  $$EXECUTE issue_5751_filter('u1')$$,
  true
) = issue_5751_result(
  $$SELECT count(*) FILTER (WHERE e.user_id = 'u1') AS filtered_aggregate_count
    FROM issue_5751_entries e
    JOIN issue_5751_series s ON s.id = e.series_id$$,
  false
) AS filter_matches_postgres;

DEALLOCATE issue_5751_having;
DEALLOCATE issue_5751_filter;

RESET client_min_messages;

RESET plan_cache_mode;

-- PostgreSQL may enforce an inner-join predicate on a parameterized inner
-- index path and remove it from the parent NestPath.joinrestrictinfo.  The
-- predicate remains in ParamPathInfo.ppi_clauses and must be inventoried when
-- AggregateScan reconstructs the DataFusion join.  Equality-only execution
-- would incorrectly count all five rows instead of the three qualifying rows.
CREATE TABLE issue_5751_ppi_series (
    id bigint PRIMARY KEY,
    threshold bigint
);
CREATE TABLE issue_5751_ppi_entries (
    id bigint PRIMARY KEY,
    series_id bigint,
    amount bigint
);

INSERT INTO issue_5751_ppi_series VALUES (1, 10), (2, 20);
INSERT INTO issue_5751_ppi_entries VALUES
    (1, 1, 5),
    (2, 1, 15),
    (3, 1, 25),
    (4, 2, 15),
    (5, 2, 25);

CREATE INDEX issue_5751_ppi_series_bm25
ON issue_5751_ppi_series USING bm25 (id, threshold)
WITH (key_field = 'id');
CREATE INDEX issue_5751_ppi_entries_bm25
ON issue_5751_ppi_entries USING bm25 (id, series_id, amount)
WITH (key_field = 'id');
CREATE INDEX issue_5751_ppi_lookup
ON issue_5751_ppi_series (id, threshold);

SET enable_hashjoin = off;
SET enable_mergejoin = off;
SET enable_seqscan = off;

SET paradedb.enable_aggregate_custom_scan = off;
SELECT issue_5751_plan_uses(
  $$SELECT count(*)
    FROM issue_5751_ppi_entries e
    JOIN issue_5751_ppi_series s
      ON s.id = e.series_id AND s.threshold < e.amount$$,
  'Nested Loop') AS lower_uses_parameterized_nestloop;

SET paradedb.enable_aggregate_custom_scan = on;
SELECT issue_5751_plan_uses(
  $$SELECT count(*)
    FROM issue_5751_ppi_entries e
    JOIN issue_5751_ppi_series s
      ON s.id = e.series_id AND s.threshold < e.amount$$,
  'ParadeDB Aggregate Scan') AS ppi_uses_aggregate_scan;

SELECT count(*) AS ppi_filtered_count
FROM issue_5751_ppi_entries e
JOIN issue_5751_ppi_series s
  ON s.id = e.series_id AND s.threshold < e.amount;

SELECT issue_5751_result(
  $$SELECT count(*)
    FROM issue_5751_ppi_entries e
    JOIN issue_5751_ppi_series s
      ON s.id = e.series_id AND s.threshold < e.amount$$,
  true
) = issue_5751_result(
  $$SELECT count(*)
    FROM issue_5751_ppi_entries e
    JOIN issue_5751_ppi_series s
      ON s.id = e.series_id AND s.threshold < e.amount$$,
  false
) AS ppi_matches_postgres;

-- Exercise generic-plan parameters and ParamPathInfo.ppi_clauses together.
-- The parameter remains a supported base filter while the cross-table
-- inequality is recovered from the parameterized lower path.
SET plan_cache_mode = force_generic_plan;
PREPARE issue_5751_ppi_generic(bigint) AS
SELECT count(*)
FROM issue_5751_ppi_entries e
JOIN issue_5751_ppi_series s
  ON s.id = e.series_id AND s.threshold < e.amount
WHERE e.amount > $1;

SELECT issue_5751_plan_uses(
  $$EXECUTE issue_5751_ppi_generic(20)$$,
  'ParadeDB Aggregate Scan') AS generic_ppi_uses_aggregate_scan;

SELECT issue_5751_result(
  $$EXECUTE issue_5751_ppi_generic(20)$$,
  true
) = issue_5751_result(
  $$SELECT count(*)
    FROM issue_5751_ppi_entries e
    JOIN issue_5751_ppi_series s
      ON s.id = e.series_id AND s.threshold < e.amount
    WHERE e.amount > 20$$,
  false
) AS generic_ppi_matches_postgres;

DEALLOCATE issue_5751_ppi_generic;
RESET plan_cache_mode;

RESET enable_hashjoin;
RESET enable_mergejoin;
RESET enable_seqscan;

DROP FUNCTION issue_5751_plan_uses(text, text), issue_5751_result(text, boolean);
DROP TABLE issue_5751_entries, issue_5751_series,
           issue_5751_ppi_entries, issue_5751_ppi_series;

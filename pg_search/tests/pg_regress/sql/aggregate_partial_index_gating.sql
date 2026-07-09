-- Regression test: the aggregate custom scan must not use a partial index when
-- the query does not imply the index's predicate.
--
-- A partial bm25 index only contains rows satisfying its WHERE predicate. If an
-- aggregate query does not imply that predicate, answering it from the index
-- would silently omit the excluded rows (e.g. undercount). The base scan already
-- rejects such cases via `predicate_implied_by`; the aggregate scan must gate
-- identically (both go through `missing_partial_index_predicate`).
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS agg_partial;
CREATE TABLE agg_partial (
    id       serial PRIMARY KEY,
    category text,
    active   boolean
);
INSERT INTO agg_partial (category, active)
SELECT (ARRAY['a', 'b', 'c'])[1 + (g % 3)], (g % 2 = 0)
FROM generate_series(1, 900) g;

-- Partial index: only rows WHERE active. `active` is itself not indexed.
CREATE INDEX agg_partial_idx ON agg_partial
USING bm25 (id, category) WITH (key_field = 'id') WHERE active;

SET paradedb.enable_aggregate_custom_scan TO on;

-- EXPLAIN helper that masks the non-deterministic index oid embedded in a
-- fallback seq-scan @@@ filter, so the declined plan is stable across runs.
CREATE OR REPLACE FUNCTION agg_partial_explain(q text) RETURNS SETOF text AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE) ' || q LOOP
    RETURN NEXT regexp_replace(r."QUERY PLAN", '"oid":\d+', '"oid":N', 'g');
  END LOOP;
END $$ LANGUAGE plpgsql;

-- Query does NOT imply the partial predicate: the aggregate scan must decline
-- (emitting the "not used" warning) and fall back to a non-index plan.
SELECT agg_partial_explain($$SELECT count(*) FROM agg_partial WHERE category === 'a'$$);
SELECT count(*) FROM agg_partial WHERE category === 'a';

-- Query DOES imply the partial predicate (AND active): the aggregate scan is used.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT count(*) FROM agg_partial WHERE category === 'a' AND active;
SELECT count(*) FROM agg_partial WHERE category === 'a' AND active;

DROP FUNCTION agg_partial_explain(text);
DROP TABLE agg_partial;

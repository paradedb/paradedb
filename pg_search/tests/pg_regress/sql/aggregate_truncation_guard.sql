-- =====================================================================
-- Runtime guard against silently truncated GROUP BY results
-- =====================================================================
-- Routing normally sends high-cardinality GROUP BYs to DataFusion, but it relies
-- on the planner's group-count estimate. When that estimate is stale/wrong the
-- query can still land on Tantivy, which caps a terms aggregation at
-- paradedb.max_term_agg_buckets and folds the dropped groups into
-- sum_other_doc_count. This test forces that case (stale statistics) and checks
-- that we error instead of returning an incomplete result.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS trunc_guard;
-- Disable autovacuum so our deliberately-stale statistics are not refreshed.
CREATE TABLE trunc_guard (id bigint, cat text) WITH (autovacuum_enabled = false);

-- Seed with only 2 distinct cat values, then ANALYZE: the planner now believes
-- cat has ~2 distinct values.
INSERT INTO trunc_guard SELECT g, 'seed_' || (g % 2) FROM generate_series(1, 100) g;
CREATE INDEX trunc_guard_idx ON trunc_guard
USING bm25 (id, (cat::pdb.literal)) WITH (key_field='id');
ANALYZE trunc_guard;

-- Add 200 genuinely-distinct cat values WITHOUT re-analyzing. The stale estimate
-- (~2) stays below the cap, so routing keeps these queries on Tantivy even
-- though the true cardinality (202) exceeds it.
INSERT INTO trunc_guard SELECT g, 'cat_' || g FROM generate_series(1000, 1199) g;

SET paradedb.max_term_agg_buckets = 10;

-- Unbounded GROUP BY: Tantivy truncates to the cap, so the guard errors instead
-- of returning a partial result.
SELECT cat, COUNT(*) FROM trunc_guard WHERE id @@@ pdb.all() GROUP BY cat ORDER BY cat;

-- Recoverable: a single grouping column bounded by a LIMIT within the cap is
-- answered correctly via Tantivy's ordered prefix, so no error. Returns the 5
-- smallest keys.
SELECT cat, COUNT(*) FROM trunc_guard WHERE id @@@ pdb.all()
GROUP BY cat ORDER BY cat LIMIT 5;

-- Once statistics are refreshed the estimate is accurate, so routing sends the
-- unbounded query to DataFusion and every group is returned — no error.
ANALYZE trunc_guard;
SELECT COUNT(*) AS groups FROM (
    SELECT cat FROM trunc_guard WHERE id @@@ pdb.all() GROUP BY cat
) s;

RESET paradedb.max_term_agg_buckets;
DROP TABLE trunc_guard;

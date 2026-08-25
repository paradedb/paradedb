-- Bitmap intersection: ParadeDB scans intersect with other indexes via a
-- BitmapIndexScan child whose TIDBitmap is probed inside the heap filter.
-- Sections: supported shapes, rejected shapes, unsupported/todo shapes.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET client_min_messages = warning;
SET paradedb.enable_filter_pushdown TO on;
SET max_parallel_workers_per_gather TO 0;

DROP TABLE IF EXISTS providers CASCADE;
CREATE TABLE providers (
    id BIGINT NOT NULL,
    description TEXT,
    gender TEXT,
    provider_type TEXT,
    specialty TEXT,
    location POINT,
    service_area CIRCLE
);

-- 10k providers on a 100x100 grid; constant "cardiology" driver term.
INSERT INTO providers (id, description, gender, provider_type, specialty, location, service_area)
SELECT i,
       'provider offering cardiology and family medicine services ' || i,
       CASE WHEN i % 2 = 0 THEN 'female' ELSE 'male' END,
       CASE WHEN i % 10 = 0 THEN 'facility' ELSE 'individual' END,
       'specialty' || (i % 50),
       point(i % 100, i / 100),
       circle(point(i % 100, i / 100), 0.4)
FROM generate_series(0, 9999) i;

CREATE INDEX providers_paradedb ON providers
    USING bm25 (id, description, gender, provider_type) WITH (key_field = 'id');
CREATE INDEX providers_location ON providers USING gist (location);
CREATE INDEX providers_service_area ON providers USING gist (service_area);
CREATE INDEX providers_specialty ON providers (specialty text_pattern_ops);
VACUUM ANALYZE providers;

-- ============================================================================
-- SUPPORTED SHAPES
-- Each asserts the Bitmap Intersection in EXPLAIN and result parity.
-- ============================================================================

-- Direct btree match: exact membership proves the clause (recheck_filters).
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty = 'specialty13'
ORDER BY id LIMIT 5;
SELECT count(*) AS btree_eq_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND specialty = 'specialty13') q;

-- Commuted clause: constant on the left.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND 'specialty13' = specialty
ORDER BY id LIMIT 5;
SELECT count(*) AS commuted_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND 'specialty13' = specialty) q;

-- Support-function derivation from a FuncExpr: starts_with becomes a pattern range.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND starts_with(specialty, 'specialty13')
ORDER BY id LIMIT 5;
SELECT count(*) AS starts_with_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND starts_with(specialty, 'specialty13')) q;

-- Support-function derivation from an OpExpr: LIKE prefix, original stays an always-filter.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty LIKE 'specialty1%'
ORDER BY id LIMIT 5;
SELECT count(*) AS like_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND specialty LIKE 'specialty1%') q;

-- GiST containment, then parity against plain heap filtering with the index gone.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND gender === 'female'
  AND location <@ circle(point(50, 50), 5)
ORDER BY id;
SELECT count(*) AS gist_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology'
      AND gender === 'female'
      AND location <@ circle(point(50, 50), 5)) q;
DROP INDEX providers_location;
SELECT count(*) AS gist_count_no_bitmap FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology'
      AND gender === 'female'
      AND location <@ circle(point(50, 50), 5)) q;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND gender === 'female'
  AND location <@ circle(point(50, 50), 5)
ORDER BY id;
CREATE INDEX providers_location ON providers USING gist (location);

-- The Aggregate Scan carries the same intersection.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT count(*) FROM providers
WHERE description === 'cardiology'
  AND gender === 'female'
  AND location <@ circle(point(50, 50), 5);
SELECT count(*) AS agg_count FROM providers
WHERE description === 'cardiology'
  AND gender === 'female'
  AND location <@ circle(point(50, 50), 5);

-- AM-level recheck: circle_ops answers && through bboxes; bbox false positives must be excluded.
SELECT count(*) AS overlap_count FROM providers
WHERE description === 'cardiology'
  AND service_area && circle(point(50, 50), 1);
DROP INDEX providers_service_area;
SELECT count(*) AS overlap_count_no_bitmap FROM providers
WHERE description === 'cardiology'
  AND service_area && circle(point(50, 50), 1);
CREATE INDEX providers_service_area ON providers USING gist (service_area);

-- Multicolumn index: quals come out in index-column order, not WHERE order.
CREATE INDEX providers_loc_area ON providers USING gist (location, service_area);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND service_area && circle(point(50, 50), 1)
  AND location <@ circle(point(50, 50), 5)
ORDER BY id;
SELECT count(*) AS multicolumn_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology'
      AND service_area && circle(point(50, 50), 1)
      AND location <@ circle(point(50, 50), 5)) q;
DROP INDEX providers_loc_area;

-- Ledger: with two candidate indexes, the more selective bitmap wins.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND specialty LIKE 'specialty1%'
  AND location <@ circle(point(50, 50), 5)
ORDER BY id LIMIT 5;

-- Parallel partial paths carry the same bitmap child (one build shared via DSA);
-- not golden-tested because the parallel plan shape depends on segment count.

-- Rescan: a generic plan rebuilds the bitmap on every execution.
SET plan_cache_mode = force_generic_plan;
PREPARE radius_search(float8) AS
SELECT count(*) FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology'
      AND gender === 'female'
      AND location <@ circle(point(50, 50), $1)) q;
EXECUTE radius_search(5);
EXECUTE radius_search(2);
EXECUTE radius_search(5);
EXECUTE radius_search(0);
DEALLOCATE radius_search;
RESET plan_cache_mode;

-- ============================================================================
-- REJECTED SHAPES
-- Correct refusals: no Bitmap Intersection, results identical to heap filtering.
-- Not golden-testable: work_mem overflow (cost gate rejects first at this scale),
-- parameterized child paths, hypothetical indexes.
-- ============================================================================

-- OR-positioned heap predicate: one arm's bitmap cannot reject rows.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND (location <@ circle(point(50, 50), 5) OR specialty LIKE 'specialty1%')
ORDER BY id LIMIT 5;

-- NOT-positioned heap predicate.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND NOT (location <@ circle(point(50, 50), 5))
ORDER BY id LIMIT 5;
SELECT count(*) AS not_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology'
      AND NOT (location <@ circle(point(50, 50), 5))) q;

-- Cost gate: an unselective bitmap is not worth building.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty LIKE 'specialty%'
ORDER BY id LIMIT 5;
SELECT count(*) AS unselective_like_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND specialty LIKE 'specialty%') q;

-- Collation gate: "C" index harvests under COLLATE "C" but not COLLATE "POSIX".
CREATE INDEX providers_specialty_c ON providers (specialty COLLATE "C");
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty COLLATE "C" > 'specialty49'
ORDER BY id LIMIT 5;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty COLLATE "POSIX" > 'specialty49'
ORDER BY id LIMIT 5;
SELECT count(*) AS collation_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND specialty COLLATE "POSIX" > 'specialty49') q;
DROP INDEX providers_specialty_c;

-- Security gate: a non-leakproof predicate under RLS stays out of the index AM
-- (superusers bypass RLS, hence the role).
CREATE ROLE regress_bitmap_rls_user;
GRANT SELECT ON providers TO regress_bitmap_rls_user;
ALTER TABLE providers ENABLE ROW LEVEL SECURITY;
CREATE POLICY providers_policy ON providers FOR SELECT USING (id >= 0);
SET ROLE regress_bitmap_rls_user;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty LIKE 'specialty13%'
ORDER BY id LIMIT 5;
SELECT count(*) AS rls_like_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND specialty LIKE 'specialty13%') q;
RESET ROLE;
DROP POLICY providers_policy ON providers;
ALTER TABLE providers DISABLE ROW LEVEL SECURITY;
REVOKE SELECT ON providers FROM regress_bitmap_rls_user;
DROP ROLE regress_bitmap_rls_user;

-- Partial index: predicate implication is not checked, so it is never a source.
BEGIN;
DROP INDEX providers_location;
CREATE INDEX providers_location_partial ON providers USING gist (location)
    WHERE provider_type = 'facility';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND provider_type === 'facility'
  AND location <@ circle(point(50, 50), 5)
ORDER BY id LIMIT 5;
ROLLBACK;

-- ============================================================================
-- UNSUPPORTED / TODO SHAPES
-- Should harvest someday; these goldens flip when the feature lands.
-- ============================================================================

-- TODO BitmapAnd: both indexes qualify; today only the best-net bitmap is used.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND specialty LIKE 'specialty13%'
  AND service_area && circle(point(50, 50), 1)
ORDER BY id LIMIT 5;

-- TODO BitmapOr: a fully indexable disjunction; today it stays a heap filter.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND (location <@ circle(point(20, 20), 3) OR location <@ circle(point(80, 80), 3))
ORDER BY id LIMIT 5;

-- TODO ScalarArrayOpExpr: = ANY could use the btree; today it stays a heap filter.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty = ANY('{specialty13,specialty14}')
ORDER BY id LIMIT 5;
SELECT count(*) AS saop_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND specialty = ANY('{specialty13,specialty14}')) q;

DROP TABLE providers CASCADE;
RESET paradedb.enable_filter_pushdown;
RESET max_parallel_workers_per_gather;
RESET client_min_messages;

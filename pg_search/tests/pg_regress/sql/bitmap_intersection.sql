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
-- Executed through the Base Scan itself (TopK), not absorbed by the Aggregate Scan.
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty = 'specialty13'
ORDER BY id LIMIT 5;

-- EXPLAIN (VERBOSE) recursive estimates treat the heap filter as a wrapper.
SET paradedb.explain_recursive_estimates = on;
EXPLAIN (VERBOSE, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty = 'specialty13'
ORDER BY id LIMIT 5;
RESET paradedb.explain_recursive_estimates;

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
-- Executed through the Base Scan itself.
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty LIKE 'specialty1%'
ORDER BY id LIMIT 5;

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
-- Executed through the Base Scan itself (window count defeats the Aggregate Scan).
SELECT count(*) OVER () AS gist_window_count FROM providers
WHERE description === 'cardiology'
  AND gender === 'female'
  AND location <@ circle(point(50, 50), 5)
LIMIT 1;
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

-- ANY over a constant array: exact membership proves the clause, same as a direct
-- btree equality match.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty = ANY('{specialty13,specialty14}')
ORDER BY id LIMIT 5;
SELECT count(*) AS saop_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND specialty = ANY('{specialty13,specialty14}')) q;

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

-- True executor rescans: a lateral nested loop rescans the inner Base Scan per
-- outer row (a generic-plan EXECUTE only creates fresh executor states).
SELECT v.k, s.id FROM (VALUES (13), (17)) v(k)
CROSS JOIN LATERAL (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND specialty = 'specialty' || v.k
    ORDER BY id LIMIT 2) s
ORDER BY v.k, s.id;
-- Rescan with a harvested bitmap: freed and rebuilt per outer row.
SELECT v.k, count(*) AS n FROM (VALUES (2), (4)) v(k)
CROSS JOIN LATERAL (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND location <@ circle(point(50, 50), 5)
    ORDER BY id LIMIT v.k) s
GROUP BY v.k ORDER BY v.k;

-- Rescan where the covered qual itself changes per outer row: the circle rides
-- in as a correlated-subquery exec param, so each rescan must free the previous
-- bitmap and rebuild with the new parameter value.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT v.x, s.id FROM (VALUES (30)) v(x)
CROSS JOIN LATERAL (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND location <@ (SELECT circle(point(v.x, 50), 5))
    ORDER BY id LIMIT 3) s
ORDER BY v.x, s.id;
SELECT v.x, s.id FROM (VALUES (30), (60)) v(x)
CROSS JOIN LATERAL (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND location <@ (SELECT circle(point(v.x, 50), 5))
    ORDER BY id LIMIT 3) s
ORDER BY v.x, s.id;
SELECT v.x, s.n FROM (VALUES (30), (60)) v(x)
CROSS JOIN LATERAL (
    SELECT count(*) AS n FROM providers
    WHERE description === 'cardiology' AND location <@ (SELECT circle(point(v.x, 50), 5))) s
ORDER BY v.x;

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

-- BitmapAnd: both predicates are indexable, so both bitmaps are intersected and
-- both filters move to recheck. Wide rows make the heap fetch expensive enough
-- that the second bitmap pays for itself.
CREATE TABLE wide_providers (
    id BIGINT, description TEXT, cat_a TEXT, cat_b TEXT, filler TEXT
);
ALTER TABLE wide_providers ALTER COLUMN filler SET STORAGE PLAIN;
INSERT INTO wide_providers
SELECT i, 'cardiology notes ' || i, 'a' || (i % 10), 'b' || (i % 10), repeat('x', 1400)
FROM generate_series(0, 4999) i;
CREATE INDEX wide_paradedb ON wide_providers
    USING bm25 (id, description) WITH (key_field = 'id');
CREATE INDEX wide_cat_a ON wide_providers (cat_a);
CREATE INDEX wide_cat_b ON wide_providers (cat_b);
VACUUM ANALYZE wide_providers;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM wide_providers
WHERE description === 'cardiology' AND cat_a = 'a3' AND cat_b = 'b3'
ORDER BY id LIMIT 5;
SELECT count(*) AS bitmap_and_count FROM (
    SELECT id FROM wide_providers
    WHERE description === 'cardiology' AND cat_a = 'a3' AND cat_b = 'b3') q;
-- Rescan with a BitmapAnd child: freed and rebuilt per outer row, re-seeding
-- through the BitmapAnd into its first leaf.
SELECT v.k, s.n FROM (VALUES ('a3'), ('a7')) v(k)
CROSS JOIN LATERAL (
    SELECT count(*) AS n FROM wide_providers
    WHERE description === 'cardiology' AND cat_a = v.k AND cat_b = 'b3') s
ORDER BY v.k;

-- Parity with the same predicates and no bitmap source available.
DROP INDEX wide_cat_a;
DROP INDEX wide_cat_b;
SELECT count(*) AS bitmap_and_count_no_bitmap FROM (
    SELECT id FROM wide_providers
    WHERE description === 'cardiology' AND cat_a = 'a3' AND cat_b = 'b3') q;
DROP TABLE wide_providers CASCADE;

-- Overlapping clause sets: overlap_ab is the better single bitmap, and overlap_bc
-- would still look like it pays for itself on top of it. Both answer cat_b, so the
-- ledger would count that clause's selectivity twice and credit overlap_bc with
-- rejecting rows overlap_ab already rejected. Sharing a clause disqualifies it.
CREATE TABLE overlap_providers (
    id BIGINT, description TEXT, cat_a TEXT, cat_b TEXT, cat_c TEXT, filler TEXT
);
ALTER TABLE overlap_providers ALTER COLUMN filler SET STORAGE PLAIN;
INSERT INTO overlap_providers
SELECT i, 'cardiology notes ' || i, 'a' || (i % 4), 'b' || ((i / 4) % 2),
       'c' || ((i / 8) % 2), repeat('x', 1400)
FROM generate_series(0, 4999) i;
CREATE INDEX overlap_paradedb ON overlap_providers
    USING bm25 (id, description) WITH (key_field = 'id');
CREATE INDEX overlap_ab ON overlap_providers (cat_a, cat_b);
CREATE INDEX overlap_bc ON overlap_providers (cat_b, cat_c);
VACUUM ANALYZE overlap_providers;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM overlap_providers
WHERE description === 'cardiology'
  AND cat_a = 'a1' AND cat_b = 'b1' AND cat_c = 'c1'
ORDER BY id LIMIT 5;
SELECT count(*) AS overlap_clause_count FROM (
    SELECT id FROM overlap_providers
    WHERE description === 'cardiology'
      AND cat_a = 'a1' AND cat_b = 'b1' AND cat_c = 'c1') q;
DROP TABLE overlap_providers CASCADE;

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

-- ALL over an array: the index cannot answer a conjunction over every element in one
-- scan, so it stays a heap filter. Results match plain heap filtering.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology' AND specialty = ALL('{specialty13,specialty13}')
ORDER BY id LIMIT 5;
SELECT count(*) AS saop_all_count FROM (
    SELECT id FROM providers
    WHERE description === 'cardiology' AND specialty = ALL('{specialty13,specialty13}')) q;

-- Cost gate: both indexes qualify, but the second bitmap only sees what the first
-- one kept, and on this narrow table those rows are worth less than the index scan
-- they cost. Only the best net bitmap is used.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND specialty LIKE 'specialty13%'
  AND service_area && circle(point(50, 50), 1)
ORDER BY id LIMIT 5;

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

-- ctid-order gate: the probe requires ctid-ascending segments; sort_by='none' never harvests.
CREATE TABLE unsorted_providers (id BIGINT, description TEXT, location POINT);
INSERT INTO unsorted_providers
SELECT i, 'cardiology notes ' || i, point(i % 100, i / 100)
FROM generate_series(0, 999) i;
CREATE INDEX unsorted_paradedb ON unsorted_providers
    USING bm25 (id, description) WITH (key_field = 'id', sort_by = 'none');
CREATE INDEX unsorted_location ON unsorted_providers USING gist (location);
VACUUM ANALYZE unsorted_providers;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM unsorted_providers
WHERE description === 'cardiology' AND location <@ circle(point(50, 5), 3)
ORDER BY id;
SELECT count(*) AS unsorted_count FROM (
    SELECT id FROM unsorted_providers
    WHERE description === 'cardiology' AND location <@ circle(point(50, 5), 3)) q;
DROP TABLE unsorted_providers CASCADE;

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

-- TODO BitmapOr: a fully indexable disjunction; today it stays a heap filter.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM providers
WHERE description === 'cardiology'
  AND (location <@ circle(point(20, 20), 3) OR location <@ circle(point(80, 80), 3))
ORDER BY id LIMIT 5;

DROP TABLE providers CASCADE;
RESET paradedb.enable_filter_pushdown;
RESET max_parallel_workers_per_gather;
RESET client_min_messages;

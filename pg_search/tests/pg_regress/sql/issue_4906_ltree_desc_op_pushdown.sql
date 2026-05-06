-- Tests for Issue #4906:
-- Push down PostgreSQL ltree descendant operator `<@` into ParadeDB BM25 index.
--
-- Required semantics:
--   lhs <@ rhs  means lhs is descendant of rhs, or equal to rhs.
--
-- Important invariant:
--   This is label-boundary hierarchy semantics, not string-prefix semantics.
--   For ancestor Top.Science:
--     Top.Science.Biology     matches
--     Top.Science             matches
--     Top.ScienceX            does NOT match
--     Top.Science_Biology     does NOT match
--     Topical.Science         does NOT match

CREATE EXTENSION IF NOT EXISTS pg_search;
CREATE EXTENSION IF NOT EXISTS ltree;

DROP TABLE IF EXISTS issue_4906_ltree CASCADE;

CREATE TABLE issue_4906_ltree (
    id BIGINT PRIMARY KEY,
    category LTREE,
    unindexed_category LTREE,
    title TEXT
);

INSERT INTO issue_4906_ltree (id, category, unindexed_category, title)
VALUES
    (1,  'Top'::ltree,                                             'Top'::ltree,                                             'root'),
    (2,  'Top.Science'::ltree,                                     'Top.Science'::ltree,                                     'science root'),
    (3,  'Top.Science.Astronomy'::ltree,                           'Top.Science.Astronomy'::ltree,                           'astronomy'),
    (4,  'Top.Science.Astronomy.Astrophysics'::ltree,               'Top.Science.Astronomy.Astrophysics'::ltree,               'astrophysics'),
    (5,  'Top.Science.Astronomy.Cosmology'::ltree,                  'Top.Science.Astronomy.Cosmology'::ltree,                  'cosmology'),
    (6,  'Top.Science.Biology'::ltree,                              'Top.Science.Biology'::ltree,                              'biology'),
    (7,  'Top.ScienceX'::ltree,                                     'Top.ScienceX'::ltree,                                     'string prefix trap sciencex'),
    (8,  'Top.Science_Biology'::ltree,                              'Top.Science_Biology'::ltree,                              'underscore sibling trap'),
    (9,  'Top.Science2'::ltree,                                     'Top.Science2'::ltree,                                     'numeric suffix sibling trap'),
    (10, 'Top.Sports'::ltree,                                       'Top.Sports'::ltree,                                       'sports'),
    (11, 'Top.Collections.Pictures.Astronomy'::ltree,               'Top.Collections.Pictures.Astronomy'::ltree,               'collection astronomy'),
    (12, 'Other.Top.Science'::ltree,                                'Other.Top.Science'::ltree,                                'contains top science but not under top'),
    (13, 'top.Science'::ltree,                                      'top.Science'::ltree,                                      'case-sensitive top'),
    (14, 'Top.Science.AstronomyStars'::ltree,                       'Top.Science.AstronomyStars'::ltree,                       'astronomystars sibling of astronomy'),
    (15, 'Top.Science.Astronomy.Stars'::ltree,                      'Top.Science.Astronomy.Stars'::ltree,                      'stars'),
    (16, 'Top.Science.Astronomy.Galaxies'::ltree,                   'Top.Science.Astronomy.Galaxies'::ltree,                   'galaxies'),
    (17, 'Top.Science.Astronomy.Astrophysics.Cluster'::ltree,       'Top.Science.Astronomy.Astrophysics.Cluster'::ltree,       'cluster'),
    (18, 'Top.Science.Astronauts'::ltree,                           'Top.Science.Astronauts'::ltree,                           'astronauts'),
    (19, NULL,                                                      NULL,                                                      'null category');

CREATE INDEX issue_4906_ltree_idx
ON issue_4906_ltree
USING bm25 (id, category, title)
WITH (key_field = 'id');

ANALYZE issue_4906_ltree;

-- Force the planner to choose Custom Scan if ParadeDB creates a valid CustomPath.
-- Keep enable_custom_scan_without_operator OFF to prove that `<@` itself was
-- pushed down as an indexed predicate, not admitted by the broad fallback GUC.
SET enable_seqscan = off;
SET paradedb.enable_custom_scan = on;
SET paradedb.enable_custom_scan_without_operator = off;

SHOW paradedb.enable_custom_scan;
SHOW paradedb.enable_custom_scan_without_operator;

CREATE OR REPLACE FUNCTION pg_temp.issue_4906_explain_text(query text)
RETURNS text
LANGUAGE plpgsql
AS $$
DECLARE
    line text;
    result text := '';
BEGIN
    FOR line IN EXECUTE format(
        'EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) %s',
        query
    )
    LOOP
        result := result
            || CASE WHEN result = '' THEN '' ELSE E'\n' END
            || line;
    END LOOP;

    RETURN result;
END;
$$;

-- 1. Core planner assertion:
--    `category <@ 'Top.Science'::ltree` must use ParadeDB Custom Scan even
--    though the SQL contains no @@@ operator and
--    enable_custom_scan_without_operator is OFF.
SELECT
    plan LIKE '%Custom Scan (ParadeDB Base Scan)%' AS uses_paradedb_custom_scan,
    plan LIKE '%Index: issue_4906_ltree_idx%' AS uses_bm25_index,
    plan LIKE '%Tantivy Query:%parse_with_field%' AS uses_parse_with_field,
    plan LIKE '%"field":"category"%' AS uses_ltree_indexed_field,
    plan LIKE '%"query_string":"Top.Science"%' AS uses_ltree_ancestor_literal
FROM (
    SELECT pg_temp.issue_4906_explain_text(
        $$SELECT id, category
          FROM issue_4906_ltree
         WHERE category <@ 'Top.Science'::ltree
         ORDER BY id$$
    ) AS plan
) s;

-- 2. The same predicate on a non-indexed ltree column must NOT be pushed into
--    ParadeDB Custom Scan. This catches accidental pushdown of the wrong field.
SELECT
    plan NOT LIKE '%Custom Scan (ParadeDB Base Scan)%' AS nonindexed_ltree_column_not_pushed_down
FROM (
    SELECT pg_temp.issue_4906_explain_text(
        $$SELECT id, unindexed_category
          FROM issue_4906_ltree
         WHERE unindexed_category <@ 'Top.Science'::ltree
         ORDER BY id$$
    ) AS plan
) s;

-- 3. Result semantics for Top.Science:
--    equality is included; descendants are included; siblings/string-prefix
--    traps are excluded; NULL is excluded.
SELECT array_agg(id ORDER BY id) AS descendant_ids_top_science
FROM issue_4906_ltree
WHERE category <@ 'Top.Science'::ltree;

-- 4. Compare pushed `<@` against PostgreSQL's no-index test analogue `^<@`.
--    The result must be byte-for-byte the same as native no-index ltree semantics.
SELECT
    (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category <@ 'Top.Science'::ltree
    ) = (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category ^<@ 'Top.Science'::ltree
    ) AS pushed_matches_ltree_noindex_semantics_top_science;

-- 5. Compare pushed `<@` with existing ParadeDB ltree facet query path.
--    This asserts that `<@` lowering is semantically equivalent to the existing
--    `category @@@ 'Top.Science'` facet search path.
SELECT
    (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category <@ 'Top.Science'::ltree
    ) = (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category @@@ 'Top.Science'
    ) AS ltree_descendant_pushdown_matches_facet_query_top_science;

-- 6. Deep ancestor semantics:
--    Top.Science.AstronomyStars must NOT match Top.Science.Astronomy,
--    because AstronomyStars is a sibling label, not a descendant label.
SELECT array_agg(id ORDER BY id) AS descendant_ids_top_science_astronomy
FROM issue_4906_ltree
WHERE category <@ 'Top.Science.Astronomy'::ltree;

SELECT
    (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category <@ 'Top.Science.Astronomy'::ltree
    ) = (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category ^<@ 'Top.Science.Astronomy'::ltree
    ) AS pushed_matches_ltree_noindex_semantics_top_science_astronomy;

SELECT
    (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category <@ 'Top.Science.Astronomy'::ltree
    ) = (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category @@@ 'Top.Science.Astronomy'
    ) AS ltree_desc_pushdown_matches_facet_query_top_science_astron;

-- 7. Explicit equality case:
--    descendant-or-equal means the ancestor itself must be returned.
SELECT array_agg(id ORDER BY id) AS equality_is_included
FROM issue_4906_ltree
WHERE category <@ 'Top.Science'::ltree
  AND category = 'Top.Science'::ltree;

-- 8. Explicit string-prefix traps:
--    These must all be excluded for ancestor Top.Science.
SELECT count(*) AS string_prefix_trap_count
FROM issue_4906_ltree
WHERE category <@ 'Top.Science'::ltree
  AND id IN (7, 8, 9, 10, 11, 12, 13);

-- 9. Top-level ancestor:
--    Descendants of Top include Top itself and everything whose first label is
--    exactly Top. It must not include Other.Top.Science, top.Science, or NULL.
SELECT array_agg(id ORDER BY id) AS descendant_ids_top
FROM issue_4906_ltree
WHERE category <@ 'Top'::ltree;

SELECT
    (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category <@ 'Top'::ltree
    ) = (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree
        WHERE category ^<@ 'Top'::ltree
    ) AS pushed_matches_ltree_noindex_semantics_top;

-- 10. Empty result should stay empty, not accidentally broaden to string prefix
--     or root facet.
SELECT coalesce(array_agg(id ORDER BY id), ARRAY[]::bigint[]) AS no_match_ids
FROM issue_4906_ltree
WHERE category <@ 'Top.Science.Astronomy.Deep'::ltree;

-- 11. Combined predicate:
--     Make sure the ltree pushdown composes with an additional heap-side qual.
--
--     `(id + 0) >= 15` is intentionally not a direct primary-key Index Cond.
--     It should be evaluated as a heap-side filter after the ltree facet query
--     has selected candidate rows from the ParadeDB index.
SET paradedb.enable_filter_pushdown = on;

SELECT array_agg(id ORDER BY id) AS combined_predicate_ids
FROM issue_4906_ltree
WHERE category <@ 'Top.Science'::ltree
  AND (id + 0) >= 15;


-- 12. Plan for the combined predicate should still use ParadeDB Custom Scan
--     and still contain both:
--       1. the ltree facet query lowered from `<@`;
--       2. the additional heap-side filter.
--
--     ParadeDB prints HeapFilter inside the serialized Tantivy Query as
--     `"heap_filter"`, not as a separate `Heap Filter:` EXPLAIN line.
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;

SELECT
    plan LIKE '%Custom Scan (ParadeDB Base Scan)%' AS combined_uses_paradedb_custom_scan,
    plan LIKE '%Tantivy Query:%parse_with_field%' AS combined_uses_parse_with_field,
    plan LIKE '%"field":"category"%' AS combined_uses_ltree_indexed_field,
    plan LIKE '%"query_string":"Top.Science"%' AS combined_uses_ltree_ancestor_literal,
    plan LIKE '%"heap_filter"%' AS combined_uses_heap_filter,
    plan LIKE '%(id + 0) >= 15%' AS combined_uses_expected_heap_filter_expr
FROM (
    SELECT pg_temp.issue_4906_explain_text(
        $$SELECT id, category
          FROM issue_4906_ltree
         WHERE category <@ 'Top.Science'::ltree
           AND (id + 0) >= 15$$
    ) AS plan
) s;

DROP TABLE issue_4906_ltree CASCADE;

-- 13. Partial BM25 index + ltree `<@` pushdown.
--
--     This is a real PostgreSQL partial index test, not merely "partial qual
--     extraction". The BM25 index only contains rows where `is_indexed` is true.
--
--     PostgreSQL may use a partial index only if the query WHERE clause implies
--     the index predicate. Here the query explicitly contains `is_indexed`,
--     so the partial BM25 index is safe to use.
DROP TABLE IF EXISTS issue_4906_ltree_partial CASCADE;

CREATE TABLE issue_4906_ltree_partial (
    id BIGINT PRIMARY KEY,
    category LTREE,
    is_indexed BOOLEAN NOT NULL,
    title TEXT NOT NULL
);

INSERT INTO issue_4906_ltree_partial (id, category, is_indexed, title)
VALUES
    (1,  'Top.Science'::ltree,                         true,  'indexed science root'),
    (2,  'Top.Science.Biology'::ltree,                 true,  'indexed biology'),
    (3,  'Top.Science.Astronomy'::ltree,               true,  'indexed astronomy'),
    (4,  'Top.Science.Astronomy.Astrophysics'::ltree,  true,  'indexed astrophysics'),
    (5,  'Top.ScienceX'::ltree,                        true,  'indexed string prefix trap'),
    (6,  'Other.Top.Science'::ltree,                   true,  'indexed other branch trap'),
    (7,  'Top.Science'::ltree,                         false, 'not indexed science root'),
    (8,  'Top.Science.Biology'::ltree,                 false, 'not indexed biology'),
    (9,  'Top.Science.Astronomy'::ltree,               false, 'not indexed astronomy'),
    (10, 'Top.Sports'::ltree,                          true,  'indexed sports'),
    (11, NULL,                                         true,  'indexed null category');

CREATE INDEX issue_4906_ltree_partial_idx
ON issue_4906_ltree_partial
USING bm25 (id, category, title)
WITH (key_field = 'id')
WHERE is_indexed;

ANALYZE issue_4906_ltree_partial;

SET enable_seqscan = off;
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;
SET paradedb.enable_custom_scan = on;
SET paradedb.enable_custom_scan_without_operator = off;
SET paradedb.enable_filter_pushdown = on;

-- 13a. Query implies the partial-index predicate and should use the partial
--      BM25 index. The original SQL contains no @@@ operator, so this also
--      verifies that `uses_index_pushdown` is enough to allow BaseScan.
SELECT
    plan LIKE '%Custom Scan (ParadeDB Base Scan)%' AS partial_index_uses_paradedb_custom_scan,
    plan LIKE '%Index: issue_4906_ltree_partial_idx%' AS partial_index_uses_expected_bm25_index,
    plan LIKE '%Tantivy Query:%parse_with_field%' AS partial_index_uses_parse_with_field,
    plan LIKE '%"field":"category"%' AS partial_index_uses_ltree_indexed_field,
    plan LIKE '%"query_string":"Top.Science"%' AS partial_index_uses_ltree_ancestor_literal
FROM (
    SELECT pg_temp.issue_4906_explain_text(
        $$SELECT id, category
            FROM issue_4906_ltree_partial
           WHERE is_indexed
             AND category <@ 'Top.Science'::ltree$$
    ) AS plan
) s;

-- 13b. Correctness: only rows inside the partial-index predicate are returned.
--      Rows 7, 8, 9 match the ltree predicate but are not indexed because
--      is_indexed = false; they must not appear.
SELECT array_agg(id ORDER BY id) AS partial_index_descendant_ids
FROM issue_4906_ltree_partial
WHERE is_indexed
  AND category <@ 'Top.Science'::ltree;

-- 13c. Compare against native no-index ltree semantics plus the same partial
--      predicate. This guards against accidentally turning `<@` into string
--      prefix semantics.
SELECT
    (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree_partial
        WHERE is_indexed
          AND category <@ 'Top.Science'::ltree
    ) = (
        SELECT array_agg(id ORDER BY id)
        FROM issue_4906_ltree_partial
        WHERE is_indexed
          AND category ^<@ 'Top.Science'::ltree
    ) AS partial_index_matches_ltree_noindex_semantics;

-- 13d. The query does NOT imply the partial-index predicate. It must not use
--      the partial BM25 index, even though the ltree predicate itself is
--      pushdown-capable.
SELECT
    plan NOT LIKE '%Custom Scan (ParadeDB Base Scan)%' AS partial_index_not_used_without_predicate
FROM (
    SELECT pg_temp.issue_4906_explain_text(
        $$SELECT id, category
            FROM issue_4906_ltree_partial
           WHERE category <@ 'Top.Science'::ltree$$
    ) AS plan
) s;

-- 13e. Contradictory predicate also must not use the partial BM25 index.
SELECT
    plan NOT LIKE '%Custom Scan (ParadeDB Base Scan)%' AS partial_index_not_used_for_contradictory_predicate
FROM (
    SELECT pg_temp.issue_4906_explain_text(
        $$SELECT id, category
            FROM issue_4906_ltree_partial
           WHERE NOT is_indexed
             AND category <@ 'Top.Science'::ltree$$
    ) AS plan
) s;

-- 14. Partial qual extraction fallback with SubPlan + ltree `<@`.
--
--     This test covers the code path:
--
--       if quals.is_none() {
--           extract each RestrictInfo individually;
--           skip only SubPlan clauses;
--           accept partial_quals if uses_our_operator OR uses_index_pushdown;
--       }
--
--     The query contains no @@@ operator. The only index-backed predicate is
--     `category <@ 'Top.Science'::ltree`, so this specifically verifies the
--     `partial_state.uses_index_pushdown` branch.
DROP TABLE IF EXISTS issue_4906_ltree_subplan_flags CASCADE;

CREATE TABLE issue_4906_ltree_subplan_flags (
    id BIGINT PRIMARY KEY,
    keep BOOLEAN NOT NULL
);

INSERT INTO issue_4906_ltree_subplan_flags (id, keep)
VALUES
    (1, true),
    (2, false),
    (3, true),
    (4, true),
    (5, true),
    (6, true),
    (7, true),
    (8, false),
    (9, true),
    (10, true),
    (11, true);

ANALYZE issue_4906_ltree_subplan_flags;

SET enable_seqscan = off;
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;
SET paradedb.enable_custom_scan = on;
SET paradedb.enable_custom_scan_without_operator = off;
SET paradedb.enable_filter_pushdown = on;

-- 14a. Result correctness with a correlated scalar SubPlan.
SELECT array_agg(p.id ORDER BY p.id) AS partial_qual_subplan_ids
FROM issue_4906_ltree_partial p
WHERE p.is_indexed
  AND p.category <@ 'Top.Science'::ltree
  AND COALESCE(
        (
            SELECT f.keep
            FROM issue_4906_ltree_subplan_flags f
            WHERE f.id = p.id
        ),
        false
      );

-- 14b. Plan assertion:
--      the SubPlan should not prevent ParadeDB from using the extracted ltree
--      pushdown qual. The ltree predicate is accepted via uses_index_pushdown,
--      not via uses_our_operator.
SELECT
    plan LIKE '%Custom Scan (ParadeDB Base Scan)%' AS partial_qual_subplan_uses_paradedb_custom_scan,
    plan LIKE '%Tantivy Query:%parse_with_field%' AS partial_qual_subplan_uses_parse_with_field,
    plan LIKE '%"field":"category"%' AS partial_qual_subplan_uses_ltree_indexed_field,
    plan LIKE '%"query_string":"Top.Science"%' AS partial_qual_subplan_uses_ltree_ancestor_literal
FROM (
    SELECT pg_temp.issue_4906_explain_text(
        $$SELECT p.id, p.category
            FROM issue_4906_ltree_partial p
           WHERE p.is_indexed
             AND p.category <@ 'Top.Science'::ltree
             AND COALESCE(
                   (
                       SELECT f.keep
                       FROM issue_4906_ltree_subplan_flags f
                       WHERE f.id = p.id
                   ),
                   false
                 )$$
    ) AS plan
) s;

RESET enable_seqscan;
RESET enable_indexscan;
RESET enable_indexonlyscan;
RESET enable_bitmapscan;

RESET paradedb.enable_custom_scan;
RESET paradedb.enable_custom_scan_without_operator;
RESET paradedb.enable_filter_pushdown;

DROP TABLE issue_4906_ltree_subplan_flags CASCADE;
DROP TABLE issue_4906_ltree_partial CASCADE;

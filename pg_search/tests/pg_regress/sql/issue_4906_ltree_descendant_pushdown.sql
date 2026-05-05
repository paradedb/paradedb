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
--     Keep this as a result-semantics test. The ORDER BY is inside array_agg
--     only for deterministic regression output.
--
--     Use `(id + 0) >= 15` rather than `id >= 15` so that this predicate is
--     not directly usable as a primary-key B-tree Index Cond.
SELECT array_agg(id ORDER BY id) AS combined_predicate_ids
FROM issue_4906_ltree
WHERE category <@ 'Top.Science'::ltree
  AND (id + 0) >= 15;

-- 12. Plan for the combined predicate should still use ParadeDB Custom Scan
--     and still contain the ltree facet query.
--
--     Important:
--     Do NOT include ORDER BY id in this plan assertion. The table has a
--     primary-key B-tree index on id, and PostgreSQL can choose that index
--     purely to satisfy ORDER BY id, even when the id predicate is not an
--     Index Cond.
--
--     Also disable ordinary PostgreSQL index scan path types here so the test
--     is about ParadeDB Custom Scan selection, not about competition with the
--     primary-key B-tree path.
SET enable_indexscan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;

SELECT
    plan LIKE '%Custom Scan (ParadeDB Base Scan)%' AS combined_uses_paradedb_custom_scan,
    plan LIKE '%Tantivy Query:%parse_with_field%' AS combined_uses_parse_with_field,
    plan LIKE '%"field":"category"%' AS combined_uses_ltree_indexed_field,
    plan LIKE '%"query_string":"Top.Science"%' AS combined_uses_ltree_ancestor_literal
FROM (
    SELECT pg_temp.issue_4906_explain_text(
        $$SELECT id, category
          FROM issue_4906_ltree
         WHERE category <@ 'Top.Science'::ltree
           AND (id + 0) >= 15$$
    ) AS plan
) s;

RESET enable_indexscan;
RESET enable_indexonlyscan;
RESET enable_bitmapscan;

RESET enable_seqscan;
RESET paradedb.enable_custom_scan;
RESET paradedb.enable_custom_scan_without_operator;

DROP TABLE issue_4906_ltree CASCADE;

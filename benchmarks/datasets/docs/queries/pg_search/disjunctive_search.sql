-- Shape: Disjunctive Search (OR Logic)
-- Join: Single Feature (Score)
-- Description: A match can occur on *either* the main table *or* the joined table (Boolean OR), but the ranking is pinned to the score of the primary ID. This is difficult because standard joins are intersections (AND); handling an OR usually requires a union or a complex filter that disrupts standard index sorting.

SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT
    f.id,
    f.title,
    paradedb.score(f.id) as score
FROM files f
LEFT JOIN documents d ON f."documentId" = d.id
WHERE
    d.parents LIKE 'PARENT_GROUP_2%'   -- Scope to a subset
    AND (
        f.title @@@ 'Title'            -- Match Local
        OR
        d.title @@@ 'Title'            -- Match Foreign
    )
ORDER BY
    score DESC                        -- Single Feature Sort (Primary Score)
LIMIT 10;

-- TODO: Currently fails with an "Unsupported query shape" error because our join cannot
-- be executed due to the `DISTINCT`.
SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT
    f.id,
    f.title,
    paradedb.score(f.id) as score
FROM files f
LEFT JOIN documents d ON f."documentId" = d.id
WHERE
    d.parents LIKE 'PARENT_GROUP_2%'   -- Scope to a subset
    AND (
        f.title @@@ 'Title'            -- Match Local
        OR
        d.title @@@ 'Title'            -- Match Foreign
    )
ORDER BY
    score DESC                        -- Single Feature Sort (Primary Score)
LIMIT 10;

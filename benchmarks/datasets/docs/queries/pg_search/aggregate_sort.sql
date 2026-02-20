-- Shape: Aggregate Sort (Computed Property)
-- Join: Single Feature (Computed Aggregate)
-- Description: The feature driving the ORDER BY does not exist until the join is performed. The user is sorting files by an aggregate of their related pages (e.g., "Most recently active" or "Most interactions"). This requires computing the aggregate (via GROUP BY or LATERAL) before the TopN can be resolved.

SET paradedb.enable_join_custom_scan TO off; SELECT
    f.id,
    f.title,
    MAX(p."createdAt") as last_activity
FROM files f
JOIN pages p ON f.id = p."fileId"
WHERE
    f.content @@@ 'Section'       -- Search Term
GROUP BY
    f.id, f.title
ORDER BY
    last_activity DESC            -- Single Feature Sort (Computed Aggregate)
LIMIT 10;

SET paradedb.enable_join_custom_scan TO on; SELECT
    f.id,
    f.title,
    MAX(p."createdAt") as last_activity
FROM files f
JOIN pages p ON f.id = p."fileId"
WHERE
    f.content @@@ 'Section'       -- Search Term
GROUP BY
    f.id, f.title
ORDER BY
    last_activity DESC            -- Single Feature Sort (Computed Aggregate)
LIMIT 10;

-- Lower bound: uses denormalized matview
SELECT
    fip.file_id,
    fip.file_title,
    MAX(fip.page_created_at) as last_activity
FROM files_inner_join_pages fip
WHERE
    fip.file_content @@@ 'Section'
GROUP BY
    fip.file_id, fip.file_title
ORDER BY
    last_activity DESC
LIMIT 10;

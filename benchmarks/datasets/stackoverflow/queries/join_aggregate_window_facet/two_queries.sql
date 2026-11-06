SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_range_partitioned_join TO on; WITH top_k AS MATERIALIZED (
    SELECT
        c.id,
        p.post_type_id,
        p.owner_user_id,
        c.score
    FROM stackoverflow_posts p
    JOIN comments c ON p.id = c.post_id
    WHERE
        p.body ||| 'code'
    ORDER BY
        c.score DESC
    LIMIT 10
),
facets AS MATERIALIZED (
    SELECT
        pdb.agg('{"terms": {"field": "post_type_id"}}') as post_type_facet,
        pdb.agg('{"terms": {"field": "owner_user_id"}}') as user_facet
    FROM stackoverflow_posts p
    JOIN comments c ON p.id = c.post_id
    WHERE
        p.body ||| 'code'
)
SELECT
    top_k.id,
    top_k.post_type_id,
    top_k.owner_user_id,
    facets.post_type_facet,
    facets.user_facet
FROM top_k
CROSS JOIN facets
ORDER BY
    top_k.score DESC;

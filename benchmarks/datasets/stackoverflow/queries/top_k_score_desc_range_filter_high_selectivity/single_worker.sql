-- force single worker
SET max_parallel_workers_per_gather = 0;

SELECT
    *,
    pdb.score(id)
FROM
    stackoverflow_posts
WHERE
    body ||| 'javascript'
    AND creation_date >= '2015-01-01'
ORDER BY
    pdb.score(id) DESC
LIMIT 10;

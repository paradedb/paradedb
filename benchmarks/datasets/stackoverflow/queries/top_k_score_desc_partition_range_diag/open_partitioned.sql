-- force single worker
SET max_parallel_workers_per_gather = 0;

SELECT
    *,
    pdb.score(id)
FROM
    stackoverflow_posts
WHERE
    body ||| 'code'
    AND id >= 20000000
ORDER BY
    pdb.score(id) DESC
LIMIT 10;

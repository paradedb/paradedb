-- force single worker
SET max_parallel_workers_per_gather = 0;

SELECT
    count(*)
FROM
    stackoverflow_posts
WHERE
    body ||| 'code'
    AND id BETWEEN 20000000 AND 30000000;

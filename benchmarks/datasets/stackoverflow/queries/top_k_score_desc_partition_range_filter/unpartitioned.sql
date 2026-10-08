SELECT
    *,
    pdb.score(id)
FROM
    stackoverflow_posts_unpartitioned
WHERE
    body ||| 'code'
    AND id BETWEEN 20000000 AND 30000000
ORDER BY
    pdb.score(id) DESC
LIMIT 10;

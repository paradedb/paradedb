SELECT
    *,
    pdb.score(id)
FROM
    stackoverflow_posts_unpartitioned
WHERE
    body ||| 'code'
ORDER BY
    pdb.score(id) DESC
LIMIT 10;

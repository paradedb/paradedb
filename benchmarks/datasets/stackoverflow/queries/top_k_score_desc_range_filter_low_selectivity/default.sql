SELECT
    *,
    pdb.score(id)
FROM
    stackoverflow_posts
WHERE
    body ||| 'code'
    AND creation_date >= '2015-01-01'
ORDER BY
    pdb.score(id) DESC
LIMIT 10;

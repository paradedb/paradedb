SELECT
    *,
    pdb.score(id)
FROM
    stackoverflow_posts
WHERE
    body ||| 'use'
    AND NOT (title ||| 'error')
    AND length(title) < 50
ORDER BY
    pdb.score(id) DESC
LIMIT 10;

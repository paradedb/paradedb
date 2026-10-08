SELECT
    *,
    pdb.score(id)
FROM
    stackoverflow_posts_unpartitioned
WHERE
    body ||| 'code'
    AND owner_user_id = 22656
ORDER BY
    pdb.score(id) DESC
LIMIT 10;

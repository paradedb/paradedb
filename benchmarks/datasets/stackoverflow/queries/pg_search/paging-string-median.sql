-- A paging query with a string paging token, roughly halfway through the dataset.
SELECT
    *
FROM
    comments
WHERE
    id @@@ pdb.all()
    AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median')
ORDER BY
    user_display_name
LIMIT 100;

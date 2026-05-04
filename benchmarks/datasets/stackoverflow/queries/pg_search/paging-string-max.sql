-- A paging query with a string paging token, at the end of the dataset.
SELECT
    *
FROM
    comments
WHERE
    id @@@ pdb.all()
    AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max')
ORDER BY
    user_display_name
LIMIT 100;

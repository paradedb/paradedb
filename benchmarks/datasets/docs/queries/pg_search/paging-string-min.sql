-- A paging query with a string paging token, at the beginning of the dataset.
SELECT
    *
FROM
    pages
WHERE
    id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min')
    AND
    id @@@ paradedb.all()
ORDER BY
    id
LIMIT 100;

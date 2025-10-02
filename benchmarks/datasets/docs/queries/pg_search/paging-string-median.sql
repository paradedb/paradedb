-- A paging query with a string paging token, roughly halfway through the dataset.
SELECT
    *
FROM
    pages
WHERE
    id @@@ paradedb.all()
    AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median')
ORDER BY
    id
LIMIT 100;

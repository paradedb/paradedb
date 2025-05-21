SELECT DISTINCT pages.*
FROM pages
JOIN files
  ON pages."fileId" = files.id
WHERE pages.content @@@ 'Single Number Reach'
  -- Matches ~0.00005 of the dataset.
  AND files."sizeInBytes" < 5 AND files.id @@@ paradedb.all()
ORDER by pages."createdAt" DESC
LIMIT 10;

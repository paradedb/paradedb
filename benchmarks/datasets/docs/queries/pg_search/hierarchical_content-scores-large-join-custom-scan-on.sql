-- Join with scores/order-by/limit, large target list.

-- Directly, without a CTE.
SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  *,
  pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'project alpha' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

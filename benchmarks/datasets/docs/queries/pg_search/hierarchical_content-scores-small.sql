-- Join with scores/order-by/limit, small target list.
SELECT
  documents.id,
  files.id,
  pages.id,
  pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

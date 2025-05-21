-- Join with no scores, small target list.
SELECT
  documents.id,
  files.id,
  pages.id
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach';

-- Join with no scores, huge target list.
SELECT
  *
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach';

-- Join with scores/order-by/limit, small target list.
SELECT
  documents.id,
  files.id,
  pages.id,
  paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

-- Join with scores/order-by/limit, huge target list.
SELECT
  *,
  paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

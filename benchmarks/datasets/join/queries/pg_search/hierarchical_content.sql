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

-- CTE to execute a smaller join before Top-N and then fetch the rest of the content after Top-N.
WITH topn AS (
  SELECT
    documents.id AS doc_id,
    files.id AS file_id,
    pages.id AS page_id,
    paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score
  FROM
    documents
    JOIN files ON documents.id = files."documentId"
    JOIN pages ON pages."fileId" = files.id
  WHERE
    documents.parents @@@ 'SFR'
    AND files.title @@@ 'collab12'
    AND pages."content" @@@ 'Single Number Reach'
  ORDER BY
    score DESC
  LIMIT 1000
)
SELECT
  d.*,
  f.*,
  p.*,
  topn.score
FROM
  topn
  JOIN documents d ON topn.doc_id = d.id
  JOIN files f ON topn.file_id = f.id
  JOIN pages p ON topn.page_id = p.id
WHERE
  topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id
ORDER BY
  topn.score DESC;

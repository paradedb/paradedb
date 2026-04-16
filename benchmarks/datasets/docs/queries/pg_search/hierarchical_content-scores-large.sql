-- Join with scores/order-by/limit, large target list.

-- Directly, without a CTE.
SET paradedb.enable_join_custom_scan TO off; SELECT
  *,
  pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents ||| 'project alpha' AND files.title ||| 'collab12' AND pages."content" ||| 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

-- CTE to execute a smaller join before Top K and then fetch the rest of the content after Top K.
WITH topk AS (
  SELECT
    documents.id AS doc_id,
    files.id AS file_id,
    pages.id AS page_id,
    pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score
  FROM
    documents
    JOIN files ON documents.id = files."documentId"
    JOIN pages ON pages."fileId" = files.id
  WHERE
    documents.parents ||| 'project alpha'
    AND files.title ||| 'collab12'
    AND pages."content" ||| 'Single Number Reach'
  ORDER BY
    score DESC
  LIMIT 1000
)
SELECT
  d.*,
  f.*,
  p.*,
  topk.score
FROM
  topk
  JOIN documents d ON topk.doc_id = d.id
  JOIN files f ON topk.file_id = f.id
  JOIN pages p ON topk.page_id = p.id
WHERE
  topk.doc_id = d.id AND topk.file_id = f.id AND topk.page_id = p.id
ORDER BY
  topk.score DESC;

-- Directly, without a CTE.
SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  *,
  pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents ||| 'project alpha' AND files.title ||| 'collab12' AND pages."content" ||| 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

-- MPP join scan
SET statement_timeout TO '120s'; SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SET paradedb.enable_mpp_join TO on; SELECT
  *,
  pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents ||| 'project alpha' AND files.title ||| 'collab12' AND pages."content" ||| 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

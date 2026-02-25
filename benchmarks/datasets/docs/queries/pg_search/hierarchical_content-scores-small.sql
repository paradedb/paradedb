-- Join with scores/order-by/limit, small target list.

SET paradedb.enable_join_custom_scan TO off; SELECT
  documents.id,
  files.id,
  pages.id,
  pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'parent group' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  documents.id,
  files.id,
  pages.id,
  pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'parent group' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

-- Lower bound: uses denormalized matview
SELECT
  djfp.doc_id,
  djfp.file_id,
  djfp.page_id,
  paradedb.score(djfp.row_id) AS score
FROM
  documents_inner_join_files_inner_join_pages djfp
WHERE
  djfp.doc_parents @@@ 'parent group' AND djfp.file_title @@@ 'collab12' AND djfp.page_content @@@ 'Single Number Reach'
ORDER BY score DESC
LIMIT 1000;

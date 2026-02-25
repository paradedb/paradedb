-- Join with no scores, small target list.

SET paradedb.enable_join_custom_scan TO off; SELECT
  documents.id,
  files.id,
  pages.id
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'parent group' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
LIMIT 5;

SET paradedb.enable_join_custom_scan TO on; SELECT
  documents.id,
  files.id,
  pages.id
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'parent group' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
LIMIT 5;

-- Lower bound: uses denormalized matview
SELECT
  djfp.doc_id,
  djfp.file_id,
  djfp.page_id
FROM
  documents_inner_join_files_inner_join_pages djfp
WHERE
  djfp.doc_parents @@@ 'parent group' AND djfp.file_title @@@ 'collab12' AND djfp.page_content @@@ 'Single Number Reach'
LIMIT 5;

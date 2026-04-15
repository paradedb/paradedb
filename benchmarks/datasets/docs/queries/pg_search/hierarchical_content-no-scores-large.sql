-- Join with no scores, large target list.

SET paradedb.enable_join_custom_scan TO off; SELECT
  *
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents ||| 'parent group' AND files.title ||| 'collab12' AND pages."content" ||| 'Single Number Reach'
LIMIT 5;

SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT
  *
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents ||| 'parent group' AND files.title ||| 'collab12' AND pages."content" ||| 'Single Number Reach'
LIMIT 5;

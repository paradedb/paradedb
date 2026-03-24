-- Join with no scores, large target list.

SET paradedb.enable_join_custom_scan TO off; EXPLAIN ANALYZE SELECT
  *
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'parent group' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
LIMIT 5;

SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; EXPLAIN ANALYZE SELECT
  *
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'parent group' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
LIMIT 5;

SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; EXPLAIN ANALYZE SELECT
  *
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'parent group' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach'
LIMIT 5;

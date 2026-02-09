-- Join with no scores, small target list.

SET paradedb.enable_mixed_fast_field_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT
  documents.id,
  files.id,
  pages.id
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach';

SET paradedb.enable_mixed_fast_field_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT
  documents.id,
  files.id,
  pages.id
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach';

-- Sortedness enabled (no join scan).
SET paradedb.enable_mixed_fast_field_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT
  documents.id,
  files.id,
  pages.id
FROM
  documents JOIN files ON documents.id = files."documentId" JOIN pages ON pages."fileId" = files.id
WHERE
  documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach';

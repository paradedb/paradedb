SELECT documents.id
FROM documents
JOIN files ON documents.id = files."documentId"
JOIN pages ON pages."fileId" = files.id
WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages."content" @@@ 'Single Number Reach';

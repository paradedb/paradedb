CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS notpdball;
CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'notpdball'
     );
CREATE INDEX idxnotpdball ON notpdball USING bm25 (id, description, category) WITH (key_field='id');


SELECT id
FROM notpdball
WHERE id @@@ paradedb.all()
ORDER BY id;
SELECT id
FROM notpdball
WHERE NOT id @@@ paradedb.all()
ORDER BY id;


SELECT a.id, b.id
FROM notpdball a,
     notpdball b
WHERE a.id = b.id AND NOT a.id @@@ paradedb.all()
   OR b.id @@@ paradedb.all()
ORDER BY a.id, b.id;

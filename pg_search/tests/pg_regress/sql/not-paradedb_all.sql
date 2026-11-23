DROP TABLE IF EXISTS notpdball;
CALL paradedb.create_paradedb_test_table(
        schema_name => 'public',
        table_name => 'notpdball'
     );
CREATE INDEX idxnotpdball ON notpdball USING paradedb (id, description, category);


SELECT id
FROM notpdball
WHERE id @@@ pdb.all()::pdb.const(0)
ORDER BY id;
SELECT id
FROM notpdball
WHERE NOT id @@@ pdb.all()::pdb.const(0)
ORDER BY id;


SELECT a.id, b.id
FROM notpdball a,
     notpdball b
WHERE a.id = b.id AND NOT a.id @@@ pdb.all()::pdb.const(0)
   OR b.id @@@ pdb.all()::pdb.const(0)
ORDER BY a.id, b.id;

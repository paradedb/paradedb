DROP TABLE IF EXISTS issue3300;
DROP TABLE IF EXISTS allowed_categories;
CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'issue3300'
     );

CREATE INDEX idxissue3000 ON issue3300 USING bm25 (id, description, rating, (category::pdb.exact), metadata) WITH (key_field='id');

CREATE TABLE allowed_categories
(
    category TEXT PRIMARY KEY
);

INSERT INTO allowed_categories (category)
VALUES ('Electronics'),
       ('Clothing');

SELECT COUNT(*)
FROM issue3300
WHERE category @@@
      paradedb.term_set(terms => ARRAY(SELECT paradedb.term('category', category) FROM allowed_categories LIMIT 5));

SELECT COUNT(*)
FROM issue3300
WHERE category @@@ pdb.term_set(terms => ARRAY(SELECT category FROM allowed_categories LIMIT 5));
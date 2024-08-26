-- TODO: transform this into some kind of regression test.

DROP TABLE IF EXISTS emails CASCADE;
CREATE TABLE emails(id int, email text);
INSERT INTO emails VALUES
  (1, '"coralee.anderson@harris.example'),
  (2, '"jane.DoE@example.com'),
  (3, '"jason.doe@example.com'),
  (4, '"jillian.DOE@example.com'),
  (5, '"JAKE.doe@example.com');

DROP SCHEMA IF EXISTS emails_bm25_idx CASCADE;
CALL paradedb.drop_bm25('emails_bm25_idx');
CALL paradedb.create_bm25(
  index_name => 'emails_bm25_idx',
  table_name => 'emails',
  key_field => 'id',
  text_fields => paradedb.field('email', tokenizer => paradedb.tokenizer('default', lowercase => true))
);

-- Normalized lowercase match returns only four results.
SELECT * FROM emails_bm25_idx.search('email:DOE');

DROP SCHEMA IF EXISTS emails_bm25_idx CASCADE;
CALL paradedb.drop_bm25('emails_bm25_idx');
CALL paradedb.create_bm25(
  index_name => 'emails_bm25_idx',
  table_name => 'emails',
  key_field => 'id',
  text_fields => paradedb.field('email', tokenizer => paradedb.tokenizer('default', lowercase => false))
);

-- Case sensitive match returns only one result.
SELECT * FROM emails_bm25_idx.search('email:DOE');

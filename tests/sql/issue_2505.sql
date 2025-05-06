-- Test for GitHub issue #2505
-- This test confirms that JOIN queries with string fast fields work correctly.
-- Previously, this would fail with an assertion error:
-- "ERROR: assertion failed: natts == self.inner.which_fast_fields.len()"

DROP TABLE IF EXISTS a;
DROP TABLE IF EXISTS b;

CREATE TABLE a (
    a_id_pk TEXT,
    content TEXT
);

CREATE TABLE b (
    b_id_pk TEXT,
    a_id_fk TEXT,
    content TEXT
);

CREATE INDEX idxa ON a USING bm25 (a_id_pk, content) WITH (key_field = 'a_id_pk');

CREATE INDEX idxb ON b USING bm25 (b_id_pk, a_id_fk, content) WITH (key_field = 'b_id_pk', 
  text_fields = '{ "a_id_fk": { "fast": true, "tokenizer": { "type": "keyword" } } }');

INSERT INTO a (a_id_pk, content) VALUES ('this-is-a-id', 'beer');
INSERT INTO b (b_id_pk, a_id_fk, content) VALUES ('this-is-b-id', 'this-is-a-id', 'wine');

VACUUM a, b;  -- needed to get Visibility Map up-to-date

-- This query used to fail with:
-- ERROR:  assertion failed: natts == self.inner.which_fast_fields.len()
-- Now it should run successfully
SELECT a.a_id_pk as my_a_id_pk, b.b_id_pk as my_b_id_pk
FROM b
JOIN a ON a.a_id_pk = b.a_id_fk
WHERE a.content @@@ 'beer' AND b.content @@@ 'wine';

DROP TABLE a;
DROP TABLE b;


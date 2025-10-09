DROP TABLE IF EXISTS rhs_typmod;
CREATE TABLE rhs_typmod(
    id serial8 not null primary key,
    t text
);
INSERT INTO rhs_typmod(t) VALUES ('hello, world');
CREATE INDEX idxrhs_typmod ON rhs_typmod USING bm25 (id, t) WITH (key_field = 'id');

-- generates ERROR as @@@ doesn't support casting to a tokenizer on the rhs
SELECT * FROM rhs_typmod WHERE t @@@ 'hello'::pdb.ngram(3, 4);

-- all of these do support a tokenizer cast on the rhs
SELECT * FROM rhs_typmod WHERE t &&& 'hello'::pdb.ngram(5, 6);
SELECT * FROM rhs_typmod WHERE t ||| 'hello'::pdb.ngram(5, 6);
SELECT * FROM rhs_typmod WHERE t ### 'hello'::pdb.ngram(5, 6);
SELECT * FROM rhs_typmod WHERE t === 'hello'::pdb.ngram(5, 6);
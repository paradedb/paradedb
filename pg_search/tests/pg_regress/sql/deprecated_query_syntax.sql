DROP TABLE IF EXISTS rhs_typmod;
CREATE TABLE rhs_typmod(
    id serial8 not null primary key,
    t text
);
INSERT INTO rhs_typmod(t) VALUES ('hello, world');
CREATE INDEX idxrhs_typmod ON rhs_typmod USING paradedb (id, t);

-- generates ERROR as @@@ doesn't support casting to a tokenizer on the rhs
SELECT * FROM rhs_typmod WHERE t @@@ 'hello'::pdb.ngram(3, 4);


DROP TABLE rhs_typmod;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM regress.mock_items WHERE description @@@ 'running shoes'::pdb.slop(2);

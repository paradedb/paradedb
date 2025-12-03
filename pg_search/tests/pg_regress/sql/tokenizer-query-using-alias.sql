DROP TABLE IF EXISTS use_alias;

SET paradedb.enable_aggregate_custom_scan TO on;

CREATE TABLE use_alias (
    id serial8 not null primary key,
    t text
);

INSERT INTO use_alias (t) VALUES ('This is a TEST');

-- generates an error
CREATE INDEX idxuse_alias ON use_alias USING bm25 (
    id,
    (t::pdb.alias(nope))
) WITH (key_field = 'id');

CREATE INDEX idxuse_alias ON use_alias USING bm25 (
    id,
    t,
    (t::pdb.literal('alias=literal')),
    (t::pdb.simple('alias=simple')),
    (t::pdb.ngram(2, 3, 'alias=ngram_2_3')),
    (t::pdb.ngram(3, 5, 'alias=ngram_3_5'))
) WITH (key_field = 'id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM use_alias WHERE t @@@ 'this is a test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM use_alias WHERE t::pdb.alias(literal) @@@ 'this is a test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM use_alias WHERE t::pdb.alias(simple) @@@ 'this is a test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM use_alias WHERE t::pdb.alias(ngram_2_3) @@@ 'this is a test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM use_alias WHERE t::pdb.alias(ngram_3_5) @@@ 'this is a test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM use_alias WHERE t::pdb.alias(no_such_alias) @@@ 'this is a test';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT count(*) FROM use_alias WHERE t::pdb.alias(no_such_alias) &&& 'this is a test';

RESET paradedb.enable_aggregate_custom_scan;

\i common/common_setup.sql

DROP TABLE IF EXISTS t;
CREATE TABLE t (id SERIAL PRIMARY KEY, indexed TEXT, nonindexed TEXT);
INSERT INTO t (indexed, nonindexed) VALUES ('hello', 'world');
CREATE INDEX t_idx ON t USING bm25 (id, indexed) WITH (key_field = 'indexed');
SELECT pdb.snippet(indexed) FROM t WHERE indexed @@@ 'hello' AND nonindexed = 'world';
SELECT pdb.snippet(nonindexed) FROM t WHERE indexed @@@ 'hello' AND nonindexed = 'world';
DROP TABLE t;

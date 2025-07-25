\i common/common_setup.sql

DROP TABLE IF EXISTS test;
CREATE TABLE test (id serial primary key, description text) PARTITION BY RANGE (id);

CREATE TABLE test_p1 PARTITION OF test FOR VALUES FROM (1) TO (5);
CREATE TABLE test_p2 PARTITION OF test FOR VALUES FROM (5) TO (10);

INSERT INTO test(description)
VALUES ('hello');

CREATE INDEX test_idx ON test USING bm25 (id, description) WITH (key_field = 'id');

SELECT * FROM test WHERE description @@@ 'hello';
DROP TABLE test;

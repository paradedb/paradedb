CREATE TABLE t (a int, b text) USING parquet;
INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c');
ALTER TABLE t RENAME TO s;
SELECT * FROM s;
SELECT * FROM t;
DROP TABLE t;
DROP TABLE s;

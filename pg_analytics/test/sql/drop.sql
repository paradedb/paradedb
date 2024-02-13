CREATE TABLE t (a int, b text) USING parquet;
DROP TABLE t;
SELECT * FROM t;
CREATE TABLE t (a int, b text) USING parquet;
CREATE TABLE s (a int, b text);
DROP TABLE s, t;
SELECT * FROM s;
SELECT * FROM t;

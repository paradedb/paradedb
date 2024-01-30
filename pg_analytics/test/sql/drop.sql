CREATE TABLE t (a int, b text) USING deltalake;
DROP TABLE t;
SELECT * FROM t;
CREATE TABLE t (a int, b text) USING deltalake;
CREATE TABLE s (a int, b text);
DROP TABLE s, t;
SELECT * FROM s;
SELECT * FROM t;

CREATE TABLE t (a int, b text) USING deltalake;
DROP TABLE t;
SELECT * FROM t;
CREATE TABLE t (a int, b text) USING deltalake;
CREATE TABLE s (a int, b text);
DROP TABLE s, t;
SELECT * FROM s;
ERROR:  relation "s" does not exist
SELECT * FROM t;
ERROR:  relation "t" does not exist

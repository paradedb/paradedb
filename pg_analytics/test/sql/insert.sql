CREATE TABLE t (a int, b int);
INSERT INTO t VALUES (1, 2);
CREATE TABLE s (a int, b int) USING deltalake;
INSERT INTO s SELECT * FROM t;
SELECT * FROM s;
DROP TABLE s, t;

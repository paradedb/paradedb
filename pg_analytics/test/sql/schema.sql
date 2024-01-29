CREATE TABLE t (a int, b text NOT NULL) USING deltalake;
INSERT INTO t values (1, 'test');
SELECT * FROM t;
DROP TABLE t;

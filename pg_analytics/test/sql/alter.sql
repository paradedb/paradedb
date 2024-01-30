CREATE TABLE t (a int, b text) USING deltalake;
ALTER TABLE t ADD COLUMN c int;
DROP TABLE t;

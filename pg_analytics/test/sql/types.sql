-- Supported Types
CREATE TABLE t (a text) USING parquet;
INSERT INTO t VALUES ('hello world');
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a varchar) USING parquet;
INSERT INTO t VALUES ('hello world');
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a char) USING parquet;
INSERT INTO t VALUES ('h');
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a smallint) USING parquet;
INSERT INTO t VALUES (1);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a integer) USING parquet;
INSERT INTO t VALUES (1);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a bigint) USING parquet;
INSERT INTO t VALUES (1);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a real) USING parquet;
INSERT INTO t VALUES (1.0);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a double precision) USING parquet;
INSERT INTO t VALUES (1.0);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a bool) USING parquet;
INSERT INTO t VALUES (true);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a numeric(5, 2)) USING parquet;
INSERT INTO t VALUES (1.01);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a timestamp) USING parquet;
INSERT INTO t VALUES ('2024-01-29 15:30:00');
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a date) USING parquet;
INSERT INTO t VALUES ('2024-01-29');
SELECT * FROM t;
DROP TABLE t;

-- Unsupported Types
CREATE TABLE t (a bytea) USING parquet;
CREATE TABLE t (a uuid) USING parquet;
CREATE TABLE t (a oid) USING parquet;
CREATE TABLE t (a json) USING parquet;
CREATE TABLE t (a jsonb) USING parquet;
CREATE TABLE t (a time) USING parquet;
CREATE TABLE t (a timetz) USING parquet;
CREATE TABLE t (a text[]) USING parquet;

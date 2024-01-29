-- Supported Types
CREATE TABLE t (a text) USING deltalake;
INSERT INTO t VALUES ('hello world');
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a varchar) USING deltalake;
INSERT INTO t VALUES ('hello world');
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a char) USING deltalake;
INSERT INTO t VALUES ('h');
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a smallint) USING deltalake;
INSERT INTO t VALUES (1);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a integer) USING deltalake;
INSERT INTO t VALUES (1);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a bigint) USING deltalake;
INSERT INTO t VALUES (1);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a real) USING deltalake;
INSERT INTO t VALUES (1.0);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a double precision) USING deltalake;
INSERT INTO t VALUES (1.0);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a bool) USING deltalake;
INSERT INTO t VALUES (true);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a numeric(5, 2)) USING deltalake;
INSERT INTO t VALUES (1.01);
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a timestamp) USING deltalake;
INSERT INTO t VALUES ('2024-01-29 15:30:00');
SELECT * FROM t;
DROP TABLE t;

CREATE TABLE t (a date) USING deltalake;
INSERT INTO t VALUES ('2024-01-29');
SELECT * FROM t;
DROP TABLE t;

-- Unsupported Types
CREATE TABLE t (a bytea) USING deltalake;
CREATE TABLE t (a uuid) USING deltalake;
CREATE TABLE t (a oid) USING deltalake;
CREATE TABLE t (a json) USING deltalake;
CREATE TABLE t (a jsonb) USING deltalake;
CREATE TABLE t (a time) USING deltalake;
CREATE TABLE t (a timetz) USING deltalake;
CREATE TABLE t (a text[]) USING deltalake;

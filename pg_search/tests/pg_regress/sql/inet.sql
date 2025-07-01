CREATE EXTENSION IF NOT EXISTS pg_search;

SET log_error_verbosity TO VERBOSE;

DROP TABLE IF EXISTS tbl_inet;
CREATE TABLE tbl_inet (id serial not null, ip inet);
CREATE INDEX idx_inet ON tbl_inet USING bm25 (id, ip) WITH (key_field = 'id');
INSERT INTO tbl_inet (ip) VALUES ('192.168.0.1');

EXPLAIN SELECT count(*) FROM tbl_inet WHERE ip @@@ '192.168.0.1';
SELECT count(*) FROM tbl_inet WHERE ip @@@ '192.168.0.1';

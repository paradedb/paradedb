CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS tbl_inet;
CREATE TABLE tbl_inet (ip inet);
CREATE INDEX idx_inet ON tbl_inet USING bm25 (ip) WITH (key_field = 'ip');
INSERT INTO tbl_inet (ip) VALUES ('192.168.0.1');
SELECT count(*) FROM tbl_inet WHERE ip @@@ '192.168.0.1';
EXPLAIN SELECT count(*) FROM tbl_inet WHERE ip @@@ '192.168.0.1';
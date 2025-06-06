CREATE TABLE tblfind_ctid (id bigint);
CREATE INDEX idxfind_ctid ON tblfind_ctid USING bm25 (id) WITH (key_field = 'id');
INSERT INTO tblfind_ctid (id) VALUES (1);
SELECT count(*) FROM paradedb.find_ctid('idxfind_ctid', '(0, 1)');
DROP TABLE IF EXISTS tblfind_ctid;
CREATE TABLE tblfind_ctid (id bigint, extra integer);
CREATE INDEX idxfind_ctid ON tblfind_ctid USING paradedb (id);
INSERT INTO tblfind_ctid (id, extra) VALUES (1, 0);
SELECT count(*) FROM paradedb.find_ctid('idxfind_ctid', '(0, 1)');
UPDATE tblfind_ctid SET extra = 1 WHERE id = 1 RETURNING ctid;
SELECT paradedb.find_ctid('idxfind_ctid', ctid) IS NULL AS hot_ctid_is_null
FROM tblfind_ctid;

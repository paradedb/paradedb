CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS tbl_inet;
CREATE TABLE tbl_inet (id int, ip inet);
CREATE INDEX idx_inet
    ON tbl_inet
    USING bm25 (id, ip)
    WITH (key_field = 'id');
INSERT INTO tbl_inet (id, ip) VALUES
    (1, '192.168.1.5/24'),
    (2, '192.168.1.5/32'),
    (3, '1.2.3.4'),
    (4, '::ffff:1.2.3.4'),
    (5, '255.255.255.255'),
    (6, '::1'),
    (7, NULL);

SELECT count(*) FROM tbl_inet WHERE ip @@@ '1.2.3.4';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT count(*) FROM tbl_inet WHERE ip @@@ '1.2.3.4';

SET paradedb.enable_custom_scan_without_operator TO on;
SET enable_seqscan TO off;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip FROM tbl_inet WHERE id @@@ paradedb.all() AND ip = '192.168.1.5/24'::inet;
SELECT ip FROM tbl_inet WHERE id @@@ paradedb.all() AND ip = '192.168.1.5/24'::inet;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip <> '1.2.3.4'::inet;
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip <> '1.2.3.4'::inet ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip < '::1'::inet;
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip < '::1'::inet ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip <= '::1'::inet;
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip <= '::1'::inet ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip > '::1'::inet;
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip > '::1'::inet ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip >= '::1'::inet;
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip >= '::1'::inet ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip BETWEEN '192.168.1.5/24'::inet AND '::1'::inet;
SELECT id FROM tbl_inet WHERE id @@@ paradedb.all() AND ip BETWEEN '192.168.1.5/24'::inet AND '::1'::inet ORDER BY id;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id, ip FROM tbl_inet WHERE id @@@ paradedb.all() ORDER BY ip ASC NULLS LAST LIMIT 6;
SELECT id, ip FROM tbl_inet WHERE id @@@ paradedb.all() ORDER BY ip ASC NULLS LAST LIMIT 6;

RESET paradedb.enable_custom_scan_without_operator;
RESET enable_seqscan;
RESET paradedb.enable_aggregate_custom_scan;

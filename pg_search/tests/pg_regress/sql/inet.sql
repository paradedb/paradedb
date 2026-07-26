CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS tbl_inet;
CREATE TABLE tbl_inet (ip inet);
CREATE INDEX idx_inet
    ON tbl_inet
    USING bm25 (ip)
    WITH (key_field = 'ip');
INSERT INTO tbl_inet (ip) VALUES
    ('192.168.1.5/24'),
    ('192.168.1.5/32'),
    ('1.2.3.4'),
    ('::ffff:1.2.3.4'),
    ('255.255.255.255'),
    ('::1');

SELECT count(*) FROM tbl_inet WHERE ip @@@ '1.2.3.4';
SELECT count(*) FROM tbl_inet WHERE ip @@@ pdb.term('1.2.3.4'::inet);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT count(*) FROM tbl_inet WHERE ip @@@ '1.2.3.4';

SET paradedb.enable_custom_scan_without_operator TO on;
SET enable_seqscan TO off;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip = '192.168.1.5/24'::inet;
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip = '192.168.1.5/24'::inet;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip <> '1.2.3.4'::inet;
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip <> '1.2.3.4'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip < '::1'::inet;
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip < '::1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip <= '::1'::inet;
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip <= '::1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip > '::1'::inet;
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip > '::1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip >= '::1'::inet;
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip >= '::1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip BETWEEN '192.168.1.5/24'::inet AND '::1'::inet;
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() AND ip BETWEEN '192.168.1.5/24'::inet AND '::1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() ORDER BY ip ASC LIMIT 4;
SELECT ip FROM tbl_inet WHERE ip @@@ paradedb.all() ORDER BY ip ASC LIMIT 4;

RESET paradedb.enable_custom_scan_without_operator;
RESET enable_seqscan;
RESET paradedb.enable_aggregate_custom_scan;

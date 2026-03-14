CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS tbl_inet;
CREATE TABLE tbl_inet (ip inet);
CREATE INDEX idx_inet ON tbl_inet USING bm25 (ip) WITH (key_field = 'ip');

INSERT INTO tbl_inet (ip) VALUES
    ('10.0.0.1'),
    ('172.16.0.1'),
    ('192.168.0.1'),
    ('198.51.100.1'),
    ('203.0.113.1');

SELECT count(*) FROM tbl_inet WHERE ip @@@ '192.168.0.1';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT count(*) FROM tbl_inet WHERE ip @@@ '192.168.0.1';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip = '192.168.0.1'::inet ORDER BY ip;
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip = '192.168.0.1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip > '192.168.0.1'::inet ORDER BY ip;
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip > '192.168.0.1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip < '192.168.0.1'::inet ORDER BY ip;
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip < '192.168.0.1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip <= '192.168.0.1'::inet ORDER BY ip;
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip <= '192.168.0.1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip >= '192.168.0.1'::inet ORDER BY ip;
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip >= '192.168.0.1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip <> '192.168.0.1'::inet ORDER BY ip;
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip <> '192.168.0.1'::inet ORDER BY ip;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip = ANY(ARRAY['10.0.0.1'::inet, '192.168.0.1'::inet]) ORDER BY ip;
SELECT ip::text FROM tbl_inet WHERE ip @@@ pdb.all() AND ip = ANY(ARRAY['10.0.0.1'::inet, '192.168.0.1'::inet]) ORDER BY ip;

RESET paradedb.enable_aggregate_custom_scan;

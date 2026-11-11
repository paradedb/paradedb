CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS inet_docs;
CREATE TABLE inet_docs (id int PRIMARY KEY, ip inet);
INSERT INTO inet_docs VALUES
  (1, '192.168.0.1'),
  (2, '192.168.0.10'),
  (3, '2001:db8::1');
CREATE INDEX inet_docs_idx ON inet_docs USING bm25 (id, ip);

SELECT id FROM inet_docs WHERE ip = '192.168.0.1'::inet ORDER BY id;
SELECT id FROM inet_docs WHERE id @@@ paradedb.term('ip', '192.168.0.1'::inet) ORDER BY id;
SELECT id FROM inet_docs WHERE ip @@@ pdb.term('192.168.0.1'::inet) ORDER BY id;
SELECT id FROM inet_docs WHERE id @@@ paradedb.term('ip', '2001:db8::1'::inet) ORDER BY id;
SELECT id FROM inet_docs WHERE ip @@@ pdb.term('2001:db8::1'::inet) ORDER BY id;

-- text literal
SELECT id FROM inet_docs WHERE id @@@ paradedb.term('ip', '192.168.0.1') ORDER BY id;

-- term_set
SELECT id FROM inet_docs
WHERE id @@@ paradedb.term_set(ARRAY[
    paradedb.term('ip', '192.168.0.1'::inet),
    paradedb.term('ip', '2001:db8::1'::inet)
]) ORDER BY id;

-- generic plan with a parameter
SET plan_cache_mode = force_generic_plan;
PREPARE inet_term(inet) AS
SELECT id FROM inet_docs WHERE id @@@ paradedb.term('ip', $1) ORDER BY id;
EXECUTE inet_term('192.168.0.1');
EXECUTE inet_term('2001:db8::1');
DEALLOCATE inet_term;
RESET plan_cache_mode;

-- without the custom scan
SET paradedb.enable_custom_scan = off;
SELECT id FROM inet_docs WHERE id @@@ paradedb.term('ip', '192.168.0.1'::inet) ORDER BY id;
RESET paradedb.enable_custom_scan;

-- a value that is not an address
SELECT id FROM inet_docs WHERE id @@@ paradedb.term('ip', 'not an ip'::text) ORDER BY id;

DROP TABLE inet_docs;

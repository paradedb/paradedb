-- Tests complex join queries with mixed fields
DROP TABLE IF EXISTS test CASCADE;

CREATE TABLE test (
   id SERIAL8 NOT NULL PRIMARY KEY,
   message TEXT,
   severity INTEGER
) WITH (autovacuum_enabled = false);

INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
INSERT INTO test (message, severity) VALUES ('beer a', 4);
INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
INSERT INTO test (message, severity) VALUES ('wine a', 6);
INSERT INTO test (message, severity) VALUES ('cheese a', 7);
INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
INSERT INTO test (message, severity) VALUES ('beer a', 4);
INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
INSERT INTO test (message, severity) VALUES ('wine a', 6);
INSERT INTO test (message, severity) VALUES ('cheese a', 7);

-- INSERT INTO test (message) SELECT 'space fillter ' || x FROM generate_series(1, 10000000) x;

CREATE INDEX idxtest ON test USING bm25(id, message, severity) WITH (key_field = 'id');
CREATE OR REPLACE FUNCTION assert(a bigint, b bigint) RETURNS bool STABLE STRICT LANGUAGE plpgsql AS $$
DECLARE
   current_txid bigint;
BEGIN
   -- Get the current transaction ID
   current_txid := txid_current();

   -- Check if the values are not equal
   IF a <> b THEN
         RAISE EXCEPTION 'Assertion failed: % <> %. Transaction ID: %', a, b, current_txid;
   END IF;

   RETURN true;
END;
$$;

SET enable_indexonlyscan to OFF;
SET enable_indexscan to OFF;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
select assert(count(*), 8), count(*) from (select id from test where message @@@ 'beer' order by severity) x limit 8;

select assert(count(*), 8), count(*) from (select id from test where message @@@ 'beer' order by severity) x limit 8;


EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
select assert(count(*), 8), count(*), max(id) from (select id from test where message @@@ 'beer' order by severity) x limit 8;

select assert(count(*), 8), count(*), max(id) from (select id from test where message @@@ 'beer' order by severity) x limit 8;


EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
select assert(count(*), 8), count(*), max(myid) from (select 12 as myid from test where message @@@ 'beer' order by severity) x limit 8;

select assert(count(*), 8), count(*), max(myid) from (select 12 as myid from test where message @@@ 'beer' order by severity) x limit 8;

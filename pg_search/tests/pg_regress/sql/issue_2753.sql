\i common/common_setup.sql

DROP TABLE IF EXISTS t;
CREATE TABLE t (id SERIAL PRIMARY KEY, domain_short TEXT, domain_long TEXT);

INSERT INTO t (domain_short, domain_long)
VALUES ('google.com', 'Google.com'), ('fb.com', 'facebook.com');

CREATE INDEX ON t USING bm25 (id, domain_short, domain_long) WITH (key_field = 'id');

SET enable_seqscan = OFF; SET enable_indexscan = OFF;

SELECT * FROM t 
WHERE lower(domain_short) = lower(domain_long)
ORDER BY id
LIMIT 5;

\i common/common_cleanup.sql

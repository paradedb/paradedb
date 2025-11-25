CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS issue_3544_test;

CREATE TABLE issue_3544_test (id SERIAL PRIMARY KEY, dates DATE, info TEXT);

INSERT INTO issue_3544_test (dates, info) VALUES ('57439-03-01', 'test');
CREATE INDEX ON issue_3544_test USING bm25 (id, dates, info) WITH (key_field = 'id');
DELETE FROM issue_3544_test;

INSERT INTO issue_3544_test (dates, info) VALUES ('2020-01-01', 'test');
CREATE INDEX ON issue_3544_test USING bm25 (id, dates, info) WITH (key_field = 'id');

EXPLAIN SELECT count(*) FROM issue_3544_test WHERE info ||| 'xyz' AND dates <@ '[1677-09-22, 2262-04-10]'::daterange;

DROP TABLE IF EXISTS issue_3544_test;

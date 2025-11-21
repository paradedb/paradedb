CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS issue_3544_test;

CREATE TABLE issue_3544_test (id SERIAL PRIMARY KEY, dates DATE);
INSERT INTO issue_3544_test (dates) VALUES ('57439-03-01');
CREATE INDEX ON issue_3544_test using bm25 (id, dates) WITH (key_field = 'id');

DROP TABLE IF EXISTS issue_3544_test;

\i common/common_setup.sql

DROP TABLE IF EXISTS filing;
CREATE TABLE filing (id serial, form text, filing TEXT,constraint filing_pkey primary key (id, form)) partition by list (form);
CREATE INDEX filing_idx ON filing USING bm25 (id, form, filing) with (key_field = 'id', text_fields = '{"form": {"fast": true}}');
CREATE TABLE filing_10_k PARTITION OF filing FOR VALUES IN ('10-K', '10-K/A');
INSERT INTO filing (form, filing) VALUES ('10-K', 'Lorem ipsum dolor sit amet, consectetur adipiscing elit.');
INSERT INTO filing (form, filing) VALUES ('10-K/A', 'Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua');
SELECT paradedb.snippet(filing) FROM filing WHERE filing @@@ 'lorem' AND form = '10-K';
DROP TABLE filing;

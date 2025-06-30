\i common/common_setup.sql

DROP TABLE IF EXISTS contacts_companies, contact_list;

CREATE TABLE contacts_companies (
    contact_id BIGINT PRIMARY KEY,
    company_id BIGINT,
    contact_name TEXT
);

CREATE TABLE contact_list (
    id BIGINT,
    list_id TEXT
);

INSERT INTO contacts_companies (contact_id, company_id, contact_name) VALUES
(17969, 1001, 'Alice'),
(17970, 1002, 'Bob'),
(17971, 1003, 'Carol'),
(17972, 1003, 'Joe'),
(17973, 1004, 'Dave');

INSERT INTO contact_list (id, list_id) VALUES (17970, 'ABCD123');
CREATE INDEX ON contacts_companies USING bm25 (contact_id, company_id, contact_name) WITH (key_field = 'contact_id');

SET enable_seqscan = off; SET enable_indexscan = off;

-- Github issue repro
SET paradedb.enable_custom_scan = on;
SELECT contact_name FROM contacts_companies
WHERE contact_name SIMILAR TO 'Alice'
AND NOT EXISTS (SELECT 1 FROM contact_list WHERE contact_id = contact_list.id);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT contact_name FROM contacts_companies
WHERE contact_name SIMILAR TO 'Alice'
AND NOT EXISTS (SELECT 1 FROM contact_list WHERE contact_id = contact_list.id);

SET paradedb.enable_custom_scan = off;
SELECT contact_name FROM contacts_companies
WHERE contact_name SIMILAR TO 'Alice'
AND NOT EXISTS (SELECT 1 FROM contact_list WHERE contact_id = contact_list.id);

-- User-reported query repro
SET paradedb.enable_custom_scan = on;
SELECT DISTINCT contact_id
FROM contacts_companies
WHERE contact_id = ANY(ARRAY[17969,17970,17971,17973])
AND NOT EXISTS (
    SELECT 1 FROM contact_list cl WHERE contact_id = cl.id
);

SET paradedb.enable_custom_scan = off;
SELECT DISTINCT contact_id
FROM contacts_companies
WHERE contact_id = ANY(ARRAY[17969,17970,17971,17973])
AND NOT EXISTS (
    SELECT 1 FROM contact_list cl WHERE contact_id = cl.id
);

DROP TABLE contacts_companies, contact_list;

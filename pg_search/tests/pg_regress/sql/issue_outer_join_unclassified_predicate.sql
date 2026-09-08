CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;

CREATE TABLE researchers (
    super_researcher_id BIGINT PRIMARY KEY,
    super_organisation_id BIGINT,
    country TEXT
);

CREATE TABLE organisations (
    super_organisation_id BIGINT PRIMARY KEY,
    super_organisation_inactive_date TIMESTAMP,
    super_organisation_name TEXT
);

INSERT INTO researchers VALUES
    (1, 10, 'US'),
    (2, 20, 'UK'),
    (3, NULL, 'CA');

INSERT INTO organisations VALUES
    (10, NULL, 'OrgA'),
    (20, '2023-01-01 00:00:00'::timestamp, NULL);

CREATE INDEX researchers_idx ON researchers
USING bm25 (super_researcher_id, super_organisation_id, country)
WITH (
    key_field = 'super_researcher_id',
    numeric_fields = '{"super_organisation_id": {"fast": true}}',
    text_fields = '{"country": {"fast": true, "tokenizer": {"type": "keyword"}}}'
);

CREATE INDEX organisations_idx ON organisations
USING bm25 (super_organisation_id, super_organisation_name)
WITH (
    key_field = 'super_organisation_id',
    text_fields = '{"super_organisation_name": {"fast": true, "tokenizer": {"type": "keyword"}}}'
);

-- TEST 1: Non-null-rejecting IS NULL on a fast field over LEFT JOIN succeeds:
EXPLAIN (COSTS OFF)
SELECT pdb.agg('{"terms": {"field": "country"}}')
FROM researchers r
LEFT JOIN organisations o ON o.super_organisation_id = r.super_organisation_id
WHERE o.super_organisation_name IS NULL;

SELECT pdb.agg('{"terms": {"field": "country"}}')
FROM researchers r
LEFT JOIN organisations o ON o.super_organisation_id = r.super_organisation_id
WHERE o.super_organisation_name IS NULL;

-- TEST 2: Non-null-rejecting IS NULL on an unindexed non-fast field produces a clear decline error:
EXPLAIN (COSTS OFF)
SELECT pdb.agg('{"terms": {"field": "country"}}')
FROM researchers r
LEFT JOIN organisations o ON o.super_organisation_id = r.super_organisation_id
WHERE o.super_organisation_inactive_date IS NULL;

SELECT pdb.agg('{"terms": {"field": "country"}}')
FROM researchers r
LEFT JOIN organisations o ON o.super_organisation_id = r.super_organisation_id
WHERE o.super_organisation_inactive_date IS NULL;

-- TEST 3: INNER JOIN with IS NULL succeeds:
EXPLAIN (COSTS OFF)
SELECT pdb.agg('{"terms": {"field": "country"}}')
FROM researchers r
JOIN organisations o ON o.super_organisation_id = r.super_organisation_id
WHERE o.super_organisation_name IS NULL;

SELECT pdb.agg('{"terms": {"field": "country"}}')
FROM researchers r
JOIN organisations o ON o.super_organisation_id = r.super_organisation_id
WHERE o.super_organisation_name IS NULL;

-- TEST 4: Disjunctive search + IS NULL on outer-joined table in pdb.agg (qgen_0 pattern)
EXPLAIN (COSTS OFF)
SELECT pdb.agg('{"terms": {"field": "country"}}')
FROM researchers r
LEFT JOIN organisations o ON o.super_organisation_id = r.super_organisation_id
WHERE (o.super_organisation_name @@@ 'OrgA') OR (o.super_organisation_name IS NULL);

SELECT pdb.agg('{"terms": {"field": "country"}}')
FROM researchers r
LEFT JOIN organisations o ON o.super_organisation_id = r.super_organisation_id
WHERE (o.super_organisation_name @@@ 'OrgA') OR (o.super_organisation_name IS NULL);

-- Clean up
DROP TABLE researchers CASCADE;
DROP TABLE organisations CASCADE;

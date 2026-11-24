-- Issue #6625: casts of grouping keys must use PostgreSQL cast semantics.
\i common/common_setup.sql
CREATE EXTENSION IF NOT EXISTS ltree;
SET paradedb.enable_aggregate_custom_scan = on;
SET datestyle = 'ISO, MDY';
SET timezone = 'UTC';

CREATE TABLE cast_groups (id INT PRIMARY KEY, d DATE, ts TIMESTAMPTZ, label VARCHAR);
INSERT INTO cast_groups VALUES
    (1, '2024-01-01', '2024-01-01 00:00:00+00', '01'),
    (2, '2024-02-02', '2024-02-02 00:00:00+00', '1'),
    (3, '2024-02-02', '2024-02-02 00:00:00+00', '1'),
    (4, NULL, NULL, NULL);
CREATE INDEX cast_groups_idx ON cast_groups USING paradedb (id, d, ts, (label::pdb.literal));
ANALYZE cast_groups;

-- The Tantivy backend writes only the key into its slot. A cast of a key in
-- the target list goes to DataFusion, and PostgreSQL evaluates the cast above.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT d::text AS dt, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY d ORDER BY 1;

SELECT d::text AS dt, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY d ORDER BY 1;

SELECT d, d::text AS dt, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY d ORDER BY d;

SELECT ts::text AS ts, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY ts ORDER BY 1;
SET timezone = 'Asia/Kolkata';
SELECT ts::text AS ts, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY ts ORDER BY 1;

SET datestyle = 'SQL, DMY';
SELECT d::text AS dt, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY d ORDER BY 1;
SET datestyle = 'ISO, MDY';

-- Casts in the GROUP BY itself also need their expression evaluated.
SELECT d::text AS dt, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY d::text ORDER BY 1;
SELECT label::int AS n, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY label ORDER BY label;
SELECT label::int AS n, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY label::int ORDER BY 1;

-- A binary-compatible relabel remains eligible for pushdown.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT label::text AS label, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY label ORDER BY 1;
SELECT label::text AS label, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY label ORDER BY 1;

-- Dropping a domain coercion would also skip its constraint.
CREATE DOMAIN cast_group_domain AS TEXT CHECK (VALUE <> '01');
SELECT label::cast_group_domain, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY label ORDER BY label;
SELECT label::cast_group_domain, COUNT(*) FROM cast_groups
WHERE id @@@ pdb.all() GROUP BY label::cast_group_domain ORDER BY 1;
DROP DOMAIN cast_group_domain;

DROP TABLE cast_groups;

-- DataFusion cannot read a key of a domain type, so a cast of that key stays
-- on the Tantivy backend, which declines it.
CREATE DOMAIN cast_group_int AS INT CHECK (VALUE > 0);
CREATE TABLE cast_domains (id INT PRIMARY KEY, p cast_group_int);
INSERT INTO cast_domains VALUES (1, 1), (2, 1), (3, 2);
CREATE INDEX cast_domains_idx ON cast_domains USING paradedb (id, p);
SELECT p::text AS p, COUNT(*) FROM cast_domains
WHERE id @@@ pdb.all() GROUP BY p ORDER BY 1;
DROP TABLE cast_domains;
DROP DOMAIN cast_group_int;

CREATE TABLE cast_json (id INT PRIMARY KEY, meta JSONB);
INSERT INTO cast_json VALUES
    (1, '{"k":"a", "n":"01"}'),
    (2, '{"k":"a", "n":"1"}'),
    (3, '{"k":"b", "n":"1"}');
CREATE INDEX cast_json_idx ON cast_json USING paradedb (id, meta)
WITH (json_fields = '{"meta": {"fast": true}}');
ANALYZE cast_json;

-- Both backends must evaluate casts above JSON paths before grouping.
SELECT (meta->'k')::text AS k, COUNT(*) FROM cast_json
WHERE id @@@ pdb.all() GROUP BY (meta->'k')::text ORDER BY 1;
SELECT (meta->>'n')::int AS n, COUNT(*) FROM cast_json
WHERE id @@@ pdb.all() GROUP BY (meta->>'n')::int ORDER BY 1;
SET paradedb.max_term_agg_buckets = 1;
EXPLAIN (COSTS OFF)
SELECT (meta->'k')::text AS k, COUNT(*) FROM cast_json
WHERE id @@@ pdb.all() GROUP BY (meta->'k')::text ORDER BY 1;
SELECT (meta->'k')::text AS k, COUNT(*) FROM cast_json
WHERE id @@@ pdb.all() GROUP BY (meta->'k')::text ORDER BY 1;
EXPLAIN (COSTS OFF)
SELECT (meta->>'n')::int AS n, COUNT(*) FROM cast_json
WHERE id @@@ pdb.all() GROUP BY (meta->>'n')::int ORDER BY 1;
SELECT (meta->>'n')::int AS n, COUNT(*) FROM cast_json
WHERE id @@@ pdb.all() GROUP BY (meta->>'n')::int ORDER BY 1;

-- A target-list cast preserves the separate JSON text groups on DataFusion.
EXPLAIN (COSTS OFF)
SELECT (meta->>'n')::int AS n, COUNT(*) FROM cast_json
WHERE id @@@ pdb.all() GROUP BY meta->>'n' ORDER BY 1, 2;
SELECT (meta->>'n')::int AS n, COUNT(*) FROM cast_json
WHERE id @@@ pdb.all() GROUP BY meta->>'n' ORDER BY 1, 2;
RESET paradedb.max_term_agg_buckets;
DROP TABLE cast_json;

-- Integer and uuid keys print the same text as PostgreSQL.
CREATE TABLE cast_ints (id INT PRIMARY KEY, i INT4, n BIGINT, u UUID);
INSERT INTO cast_ints VALUES
    (1, -5, -5000000000, 'A0EEBC99-9C0B-4EF8-BB6D-6BB9BD380A11'),
    (2, 10, 10, 'b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11'),
    (3, -5, -5000000000, 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11'),
    (4, NULL, NULL, NULL);
CREATE INDEX cast_ints_idx ON cast_ints USING paradedb (id, i, n, u);
EXPLAIN (COSTS OFF, VERBOSE)
SELECT i::text AS i, COUNT(*) FROM cast_ints
WHERE id @@@ pdb.all() GROUP BY i::text ORDER BY 1;
SELECT i::text AS i, COUNT(*) FROM cast_ints
WHERE id @@@ pdb.all() GROUP BY i::text ORDER BY 1;
SELECT n::text AS n, COUNT(*) FROM cast_ints
WHERE id @@@ pdb.all() GROUP BY n ORDER BY n;
EXPLAIN (COSTS OFF, VERBOSE)
SELECT u::text AS u, COUNT(*) FROM cast_ints
WHERE id @@@ pdb.all() GROUP BY u::text ORDER BY 1;
SELECT u::text AS u, COUNT(*) FROM cast_ints
WHERE id @@@ pdb.all() GROUP BY u::text ORDER BY 1;
DROP TABLE cast_ints;

-- PostgreSQL evaluates a cast above unnest() on the element value.
CREATE TABLE cast_arrays (id INT PRIMARY KEY, a INT4[]);
INSERT INTO cast_arrays VALUES (1, '{-1,2}'), (2, '{2,10}'), (3, NULL);
CREATE INDEX cast_arrays_idx ON cast_arrays USING paradedb (id, a);
SELECT unnest(a)::text AS v, COUNT(*) FROM cast_arrays
WHERE id @@@ pdb.all() GROUP BY 1 ORDER BY 1;
SELECT unnest(a)::text AS v, COUNT(*) FROM cast_arrays
WHERE id @@@ pdb.all() GROUP BY unnest(a) ORDER BY 1;
DROP TABLE cast_arrays;

-- An indexed cast already stores the grouping expression's value.
CREATE TABLE cast_indexed (id INT PRIMARY KEY, label TEXT, meta JSONB);
INSERT INTO cast_indexed VALUES
    (1, '01', '{"n":"01"}'), (2, '1', '{"n":"1"}'), (3, '7', '{"n":"7"}'), (4, NULL, NULL);
CREATE INDEX cast_indexed_idx ON cast_indexed USING paradedb (
    id, (label::pdb.literal), meta,
    ((label::int)::pdb.alias('label_int')),
    (((meta->>'n')::int)::pdb.alias('n_int')));
ANALYZE cast_indexed;
EXPLAIN (COSTS OFF)
SELECT label::int AS n, COUNT(*) FROM cast_indexed
WHERE id @@@ pdb.all() GROUP BY label::int ORDER BY 1;
SELECT label::int AS n, COUNT(*) FROM cast_indexed
WHERE id @@@ pdb.all() GROUP BY label::int ORDER BY 1;
SET paradedb.max_term_agg_buckets = 1;
-- DataFusion also groups on an indexed cast of a JSON path.
EXPLAIN (COSTS OFF)
SELECT (meta->>'n')::int AS n, COUNT(*) FROM cast_indexed
WHERE id @@@ pdb.all() GROUP BY (meta->>'n')::int ORDER BY 1;
SELECT (meta->>'n')::int AS n, COUNT(*) FROM cast_indexed
WHERE id @@@ pdb.all() GROUP BY (meta->>'n')::int ORDER BY 1;
-- DataFusion reads the column for a cast directly over it, so it declines.
EXPLAIN (COSTS OFF)
SELECT label::int AS n, COUNT(*) FROM cast_indexed
WHERE id @@@ pdb.all() GROUP BY label::int ORDER BY 1;
SELECT label::int AS n, COUNT(*) FROM cast_indexed
WHERE id @@@ pdb.all() GROUP BY label::int ORDER BY 1;
RESET paradedb.max_term_agg_buckets;
DROP TABLE cast_indexed;

-- The existing ltree-to-text decoder remains usable.
CREATE TABLE cast_paths (id INT PRIMARY KEY, path LTREE);
INSERT INTO cast_paths VALUES (1, 'Top.Science'), (2, 'Top.Science'), (3, 'Top.Sport');
CREATE INDEX cast_paths_idx ON cast_paths USING paradedb (id, path);
EXPLAIN (COSTS OFF, VERBOSE)
SELECT path::text, COUNT(*) FROM cast_paths
WHERE id @@@ pdb.all() GROUP BY path::text ORDER BY 1;
SELECT path::text, COUNT(*) FROM cast_paths
WHERE id @@@ pdb.all() GROUP BY path::text ORDER BY 1;
-- A further relabel must not bypass the decoder's text-only restriction.
SELECT path::text::varchar, COUNT(*) FROM cast_paths
WHERE id @@@ pdb.all() GROUP BY path::text::varchar ORDER BY 1;
ANALYZE cast_paths;
SET paradedb.max_term_agg_buckets = 1;
-- DataFusion projects a cast over a plain ltree key, but declines a cast key.
SELECT path::text, COUNT(*) FROM cast_paths
WHERE id @@@ pdb.all() GROUP BY path ORDER BY 1;
EXPLAIN (COSTS OFF)
SELECT path::text, COUNT(*) FROM cast_paths
WHERE id @@@ pdb.all() GROUP BY path::text ORDER BY 1;
SELECT path::text, COUNT(*) FROM cast_paths
WHERE id @@@ pdb.all() GROUP BY path::text ORDER BY 1;
DROP TABLE cast_paths;
RESET paradedb.max_term_agg_buckets;
RESET paradedb.enable_aggregate_custom_scan;
RESET datestyle;
RESET timezone;
\i common/common_cleanup.sql

-- An MPP join can decode grouped string keys on both sides of a partial aggregate.
-- The second decode must rebuild its fast-field helper if the first covered only one key.
SET client_min_messages = warning;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE mpp_decode_users (id INT PRIMARY KEY, name TEXT, age INT, metadata JSONB);
CREATE TABLE mpp_decode_products (id INT PRIMARY KEY, name TEXT, age INT, metadata JSONB);

CREATE INDEX mpp_decode_users_idx ON mpp_decode_users USING paradedb
    (id, (name::pdb.literal), age, (metadata::pdb.unicode_words('columnar=true')))
    WITH (target_segment_count = 4);
CREATE INDEX mpp_decode_products_idx ON mpp_decode_products USING paradedb
    (id, (name::pdb.literal), age, (metadata::pdb.unicode_words('columnar=true')))
    WITH (target_segment_count = 4);

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO mpp_decode_users
SELECT i, (ARRAY['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy'])[1 + i % 7],
       i % 30,
       CASE WHEN i % 5 = 0 THEN NULL
            WHEN i % 5 = 1 THEN '{}'::jsonb
            ELSE jsonb_build_object('brand', (ARRAY['apple', 'samsung', 'sony', 'lg'])[1 + i % 4]) END
FROM generate_series(1, 25) AS i;
INSERT INTO mpp_decode_products
SELECT i, 'p', i % 30 + 5, '{}'::jsonb FROM generate_series(1, 25) AS i;
INSERT INTO mpp_decode_users
SELECT i, (ARRAY['alice', 'bob', 'cloe', 'sally', 'brandy', 'brisket', 'anchovy'])[1 + i % 7],
       i % 30,
       CASE WHEN i % 5 = 0 THEN NULL
            WHEN i % 5 = 1 THEN '{}'::jsonb
            ELSE jsonb_build_object('brand', (ARRAY['apple', 'samsung', 'sony', 'lg'])[1 + i % 4]) END
FROM generate_series(26, 50) AS i;
INSERT INTO mpp_decode_products
SELECT i, 'p', i % 30 + 5, '{}'::jsonb FROM generate_series(26, 50) AS i;
RESET paradedb.global_mutable_segment_rows;
ANALYZE mpp_decode_users;
ANALYZE mpp_decode_products;

SET paradedb.enable_aggregate_custom_scan = on;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_filter_pushdown = off;
SET paradedb.add_doc_count_to_aggs = on;
SET paradedb.mpp_min_rows = 0;
SET parallel_leader_participation = off;

SET max_parallel_workers_per_gather = 0;
CREATE TEMP TABLE mpp_decode_reference AS
SELECT u.name, SUM(u.age) AS total_age, AVG(u.age) AS average_age,
       u.metadata->>'brand' AS brand
FROM mpp_decode_users u JOIN mpp_decode_products p
    ON u.id = p.id AND u.age < p.age
WHERE NOT (u.name === 'bob')
GROUP BY u.name, u.metadata->>'brand';

SET max_parallel_workers_per_gather = 4;
EXPLAIN (COSTS OFF)
SELECT u.name, SUM(u.age), AVG(u.age), u.metadata->>'brand'
FROM mpp_decode_users u JOIN mpp_decode_products p
    ON u.id = p.id AND u.age < p.age
WHERE NOT (u.name === 'bob')
GROUP BY u.name, u.metadata->>'brand';

CREATE TEMP TABLE mpp_decode_actual AS
SELECT u.name, SUM(u.age) AS total_age, AVG(u.age) AS average_age,
       u.metadata->>'brand' AS brand
FROM mpp_decode_users u JOIN mpp_decode_products p
    ON u.id = p.id AND u.age < p.age
WHERE NOT (u.name === 'bob')
GROUP BY u.name, u.metadata->>'brand';

SELECT (SELECT COUNT(*) FROM mpp_decode_reference) AS groups,
       (SELECT COUNT(*) FROM (
           (SELECT * FROM mpp_decode_reference EXCEPT ALL SELECT * FROM mpp_decode_actual)
           UNION ALL
           (SELECT * FROM mpp_decode_actual EXCEPT ALL SELECT * FROM mpp_decode_reference)
       ) differences) AS differences;

DROP TABLE mpp_decode_users, mpp_decode_products;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.enable_filter_pushdown;
RESET paradedb.add_doc_count_to_aggs;
RESET paradedb.mpp_min_rows;
RESET parallel_leader_participation;
RESET max_parallel_workers_per_gather;
RESET client_min_messages;

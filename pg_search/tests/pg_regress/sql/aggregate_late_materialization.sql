-- Coverage for the aggregate late-materialization path behind
-- paradedb.enable_aggregate_late_materialization. Default off keeps aggregates
-- eager; this flips it on so the deferred path (serial and MPP) does not rot.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_aggregate_late_materialization TO on;

CREATE TABLE alm_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT
);
CREATE TABLE alm_tags (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    tag_name TEXT
);

INSERT INTO alm_products (description, category, price) VALUES
    ('laptop fast', 'Electronics', 999.99),
    ('laptop gaming', 'Electronics', 1299.99),
    ('shoes running', 'Sports', 89.99),
    ('shoes trail', 'Sports', 119.99),
    ('jacket winter', 'Clothing', 129.99);
INSERT INTO alm_tags (product_id, tag_name) VALUES
    (1, 'tech'), (2, 'tech'), (3, 'fitness'), (4, 'fitness'), (5, 'outdoor');

CREATE INDEX alm_products_idx ON alm_products
USING bm25 (id, description, category, price)
WITH (key_field='id', text_fields='{"description": {}, "category": {"fast": true}}', numeric_fields='{"price": {"fast": true}}');
CREATE INDEX alm_tags_idx ON alm_tags
USING bm25 (id, product_id, tag_name)
WITH (key_field='id', numeric_fields='{"product_id": {"fast": true}}', text_fields='{"tag_name": {"fast": true}}');

-- Delete a matched row so visibility actually filters. The deleted product must
-- not appear in the aggregate.
DELETE FROM alm_products WHERE id = 2;

-- Serial: the deferred path puts a VisibilityFilterExec above the join.
SET max_parallel_workers_per_gather TO 0;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*)
FROM alm_products p JOIN alm_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*)
FROM alm_products p JOIN alm_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- Same result must hold on the MPP path.
SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

SELECT p.category, COUNT(*)
FROM alm_products p JOIN alm_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

DROP TABLE alm_products, alm_tags;

-- A nested level on an array field reads the join a second time. Both reads have to
-- agree on the group key's type, or the array level's buckets belong to no parent and
-- drop out of the result.
CREATE TABLE alm_posts (
    id SERIAL PRIMARY KEY,
    title TEXT,
    author TEXT,
    labels TEXT[]
);
CREATE TABLE alm_views (
    id SERIAL PRIMARY KEY,
    post_id INTEGER
);

INSERT INTO alm_posts (title, author, labels)
SELECT 'post ' || i,
       (ARRAY['ann', 'bob', 'cid'])[1 + i % 3],
       ARRAY[(ARRAY['red', 'green', 'blue'])[1 + i % 3], 'all']
FROM generate_series(1, 60) AS i;
INSERT INTO alm_views (post_id) SELECT id FROM alm_posts;

CREATE INDEX alm_posts_idx ON alm_posts
USING bm25 (id, title, author, labels)
WITH (key_field='id', text_fields='{"title": {}, "author": {"fast": true}, "labels": {"fast": true}}');
CREATE INDEX alm_views_idx ON alm_views
USING bm25 (id, post_id)
WITH (key_field='id', numeric_fields='{"post_id": {"fast": true}}');

SET max_parallel_workers_per_gather TO 0;
-- Pinned so the shape under test does not depend on what the placement rule picks.
SET paradedb.defer_string_decode TO on;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "p.author", "order": {"_key": "asc"}, "size": 10}, "aggs": {"by_label": {"terms": {"field": "p.labels", "order": {"_key": "asc"}, "size": 10}}}}')
FROM alm_posts p JOIN alm_views v ON p.id = v.post_id
WHERE p.title @@@ 'post';

SELECT pdb.agg('{"terms": {"field": "p.author", "order": {"_key": "asc"}, "size": 10}, "aggs": {"by_label": {"terms": {"field": "p.labels", "order": {"_key": "asc"}, "size": 10}}}}')
FROM alm_posts p JOIN alm_views v ON p.id = v.post_id
WHERE p.title @@@ 'post';

RESET paradedb.defer_string_decode;
DROP TABLE alm_posts, alm_views;

\i common/common_setup.sql

-- A HOT update leaves the index pointing at the old line pointer, and VACUUM then turns
-- that line pointer into a redirect. The join scan fetches its final rows by ctid, so it
-- has to hand the fetch the ctid of the live tuple, not the redirect.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;
SET paradedb.enable_join_custom_scan TO on;

DROP TABLE IF EXISTS hot_users CASCADE;
DROP TABLE IF EXISTS hot_orders CASCADE;

CREATE TABLE hot_users (
    id INTEGER PRIMARY KEY,
    name TEXT,
    age INTEGER,
    -- In no index at all, so an update of it is a HOT update.
    rating INTEGER
);

CREATE TABLE hot_orders (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    note TEXT
);

INSERT INTO hot_users (id, name, age, rating) VALUES
(1, 'alice', 30, 1),
(2, 'bob', 41, 1),
(3, 'alice', 25, 1),
(4, 'bob', 37, 1),
(5, 'carol', 52, 1),
(6, 'bob', 29, 1);

INSERT INTO hot_orders (id, user_id, note) VALUES
(10, 1, 'first order'),
(11, 2, 'second order'),
(12, 2, 'third order'),
(13, 3, 'fourth order'),
(14, 4, 'fifth order'),
(15, 5, 'sixth order'),
(16, 6, 'seventh order');

CREATE INDEX hot_users_idx ON hot_users USING paradedb (id, name, age)
WITH (
    text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}',
    numeric_fields = '{"age": {"fast": true}}'
);

CREATE INDEX hot_orders_idx ON hot_orders USING paradedb (id, user_id, note)
WITH (
    numeric_fields = '{"user_id": {"fast": true}}'
);

UPDATE hot_users SET rating = rating + 1 WHERE id IN (2, 4);
VACUUM (TRUNCATE false) hot_users;

-- Both bobs with a HOT-updated row must still come back.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT u.id, u.name, u.rating, o.note
FROM hot_users u
JOIN hot_orders o ON o.user_id = u.id
WHERE u.name @@@ 'bob'
ORDER BY u.id, o.id
LIMIT 10;

SELECT u.id, u.name, u.rating, o.note
FROM hot_users u
JOIN hot_orders o ON o.user_id = u.id
WHERE u.name @@@ 'bob'
ORDER BY u.id, o.id
LIMIT 10;

-- Sorted by a fast text column, so the top-k runs against the deferred column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT u.id, u.name, u.rating, o.note
FROM hot_users u
JOIN hot_orders o ON o.user_id = u.id
WHERE u.name @@@ 'bob OR alice'
ORDER BY u.name, u.id, o.id
LIMIT 4;

SELECT u.id, u.name, u.rating, o.note
FROM hot_users u
JOIN hot_orders o ON o.user_id = u.id
WHERE u.name @@@ 'bob OR alice'
ORDER BY u.name, u.id, o.id
LIMIT 4;

-- The same two queries without the join scan, as the reference.
SET paradedb.enable_join_custom_scan TO off;

SELECT u.id, u.name, u.rating, o.note
FROM hot_users u
JOIN hot_orders o ON o.user_id = u.id
WHERE u.name @@@ 'bob'
ORDER BY u.id, o.id
LIMIT 10;

SELECT u.id, u.name, u.rating, o.note
FROM hot_users u
JOIN hot_orders o ON o.user_id = u.id
WHERE u.name @@@ 'bob OR alice'
ORDER BY u.name, u.id, o.id
LIMIT 4;

DROP TABLE hot_orders;
DROP TABLE hot_users;

RESET paradedb.enable_join_custom_scan;
RESET enable_indexscan;
RESET max_parallel_workers_per_gather;

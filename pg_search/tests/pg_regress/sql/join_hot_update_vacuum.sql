\i common/common_setup.sql

-- A HOT update leaves the index pointing at the old line pointer, and VACUUM then turns
-- that line pointer into a redirect on an all-visible page. The join scan fetches its final
-- rows by that ctid, so the fetch has to follow the redirect.

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
    key_field = 'id',
    text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}',
    numeric_fields = '{"age": {"fast": true}}'
);

CREATE INDEX hot_orders_idx ON hot_orders USING paradedb (id, user_id, note)
WITH (
    key_field = 'id',
    numeric_fields = '{"user_id": {"fast": true}}'
);

CREATE EXTENSION IF NOT EXISTS pageinspect;
CREATE EXTENSION IF NOT EXISTS pg_visibility;

CREATE TEMP TABLE hot_users_roots AS
SELECT ctid AS root_ctid FROM hot_users WHERE id IN (2, 4);

UPDATE hot_users SET rating = rating + 1 WHERE id IN (2, 4);
VACUUM (FREEZE, TRUNCATE false) hot_users;

-- The scan only keeps a root ctid while the page is all-visible, and the root only loses its
-- tuple once it is a redirect. If either is false here, the queries below prove nothing.
SELECT bool_and(all_visible) AS page_all_visible
FROM pg_visibility_map('hot_users')
WHERE blkno IN (SELECT (root_ctid::text::point)[0]::bigint FROM hot_users_roots);

SELECT bool_and(p.lp_flags = 2) AS roots_are_redirects
FROM hot_users_roots r
JOIN heap_page_items(get_raw_page('hot_users', (r.root_ctid::text::point)[0]::integer)) p
  ON p.lp = (r.root_ctid::text::point)[1]::smallint;

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

-- The nullable side of an outer join is checked for visibility inside its own scan.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT o.id, o.note, u.id, u.name, u.rating
FROM hot_orders o
LEFT JOIN hot_users u ON o.user_id = u.id
WHERE o.note @@@ 'order'
ORDER BY o.id
LIMIT 10;

SELECT o.id, o.note, u.id, u.name, u.rating
FROM hot_orders o
LEFT JOIN hot_users u ON o.user_id = u.id
WHERE o.note @@@ 'order'
ORDER BY o.id
LIMIT 10;

-- The same three queries without the join scan, as the reference.
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

SELECT o.id, o.note, u.id, u.name, u.rating
FROM hot_orders o
LEFT JOIN hot_users u ON o.user_id = u.id
WHERE o.note @@@ 'order'
ORDER BY o.id
LIMIT 10;

DROP TABLE hot_orders;
DROP TABLE hot_users;
DROP EXTENSION IF EXISTS pg_visibility;
DROP EXTENSION IF EXISTS pageinspect;

RESET paradedb.enable_join_custom_scan;

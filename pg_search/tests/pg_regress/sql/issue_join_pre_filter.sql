-- Setup
DROP TABLE IF EXISTS issue_join_pre_filter_users CASCADE;
DROP TABLE IF EXISTS issue_join_pre_filter_posts CASCADE;

CREATE TABLE issue_join_pre_filter_users (
    id integer PRIMARY KEY,
    reputation integer
);
CREATE INDEX ON issue_join_pre_filter_users USING bm25 (id, reputation) WITH (key_field=id, sort_by='id ASC NULLS FIRST');

CREATE TABLE issue_join_pre_filter_posts (
    id integer PRIMARY KEY,
    title text,
    owner_user_id integer
);
CREATE INDEX ON issue_join_pre_filter_posts USING bm25 (id, (title::pdb.unicode_words('columnar=true')), owner_user_id) WITH (key_field=id, sort_by='owner_user_id ASC NULLS FIRST');

INSERT INTO issue_join_pre_filter_users SELECT i, 200 FROM generate_series(1, 10000) i;
INSERT INTO issue_join_pre_filter_posts SELECT i, 'how using get create', i % 1000 + 1 FROM generate_series(1, 10000) i;

ANALYZE issue_join_pre_filter_users;
ANALYZE issue_join_pre_filter_posts;

SET max_parallel_workers_per_gather = 2;
SET paradedb.enable_join_custom_scan TO on;
SET work_mem TO '4GB';

-- Note: This test reliably reproduces the pre-filter "Column 0 not fetched" panic.
-- It may or may not exercise the second fix (ensuring pdb.score is projected
-- when a parent Result node needs it) depending on whether the planner
-- chooses a plan shape with a Result node above the Custom Scan.
EXPLAIN (COSTS OFF, TIMING OFF) SELECT
    p.id,
    p.title,
    pdb.score(p.id) as relevance
FROM issue_join_pre_filter_posts p
JOIN issue_join_pre_filter_users u ON p.owner_user_id = u.id
WHERE
    p.title ||| 'how using get create'
    AND u.id @@@ pdb.all()
    AND u.reputation > 100
ORDER BY
    relevance DESC
LIMIT 10;

-- Reproduce the bug
SELECT
    p.id,
    p.title,
    pdb.score(p.id) as relevance
FROM issue_join_pre_filter_posts p
JOIN issue_join_pre_filter_users u ON p.owner_user_id = u.id
WHERE
    p.title ||| 'how using get create'
    AND u.id @@@ pdb.all()
    AND u.reputation > 100
ORDER BY
    relevance DESC
LIMIT 10;

-- Teardown
DROP TABLE issue_join_pre_filter_users CASCADE;
DROP TABLE issue_join_pre_filter_posts CASCADE;

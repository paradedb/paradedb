-- Create tables for the Stack Overflow dataset.

DROP TABLE IF EXISTS stackoverflow_posts CASCADE;
DROP TABLE IF EXISTS badges CASCADE;
DROP TABLE IF EXISTS comments CASCADE;
DROP TABLE IF EXISTS users CASCADE;

CREATE TABLE stackoverflow_posts (
    id INTEGER PRIMARY KEY,
    title VARCHAR,
    body TEXT,
    accepted_answer_id INTEGER,
    answer_count INTEGER,
    comment_count INTEGER,
    community_owned_date TIMESTAMP,
    creation_date TIMESTAMP,
    favorite_count INTEGER,
    last_activity_date TIMESTAMP,
    last_edit_date TIMESTAMP,
    last_editor_display_name VARCHAR,
    last_editor_user_id INTEGER,
    owner_display_name VARCHAR,
    owner_user_id INTEGER,
    parent_id INTEGER,
    post_type_id SMALLINT,
    score INTEGER,
    tags VARCHAR,
    view_count INTEGER,
    -- NUMERIC shapes for aggregate pushdown benchmarks, one per storage
    -- format. Generated so the sampled CSVs stay unchanged: COPY skips
    -- generated columns. amount78 mimics uint256-style token amounts.
    amount15 NUMERIC(15, 2) GENERATED ALWAYS AS (view_count::numeric / 100) STORED,
    amount78 NUMERIC(78, 0) GENERATED ALWAYS AS ('1e70'::numeric + view_count) STORED,
    -- Deterministic full-entropy UUID join key mirroring owner_user_id.
    -- Explicit ::text cast avoids STABLE textanycat; preserves NULL when owner_user_id IS NULL.
    owner_user_uuid UUID GENERATED ALWAYS AS (md5('user-' || owner_user_id::text)::uuid) STORED
);

CREATE TABLE badges (
    id INTEGER PRIMARY KEY,
    name VARCHAR,
    date TIMESTAMP,
    user_id INTEGER,
    class INTEGER,
    tag_based BOOLEAN,
    -- Deterministic full-entropy UUID join key mirroring user_id.
    user_uuid UUID GENERATED ALWAYS AS (md5('user-' || user_id::text)::uuid) STORED
);

CREATE TABLE comments (
    id INTEGER PRIMARY KEY,
    text TEXT,
    creation_date TIMESTAMP,
    post_id INTEGER,
    user_id INTEGER,
    user_display_name VARCHAR,
    score INTEGER
);

CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    display_name VARCHAR,
    about_me TEXT,
    age INTEGER,
    creation_date TIMESTAMP,
    last_access_date TIMESTAMP,
    location VARCHAR,
    reputation INTEGER,
    up_votes INTEGER,
    down_votes INTEGER,
    views INTEGER,
    profile_image_url VARCHAR,
    website_url VARCHAR,
    -- Deterministic full-entropy UUID join key mirroring id.
    user_uuid UUID GENERATED ALWAYS AS (md5('user-' || id::text)::uuid) STORED
);

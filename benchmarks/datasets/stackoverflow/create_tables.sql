-- Create tables for the Stack Overflow dataset.

DROP TABLE IF EXISTS comments CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS stackoverflow_posts CASCADE;

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
    view_count INTEGER
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
    website_url VARCHAR
);

CREATE TABLE badges (
    id INTEGER PRIMARY KEY,
    name VARCHAR,
    date TIMESTAMP,
    user_id INTEGER,
    class INTEGER,
    tag_based BOOLEAN
);

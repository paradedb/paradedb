-- Create tables for the Stack Overflow dataset.
-- Schema is based on the standard Stack Overflow data dump.
-- Adjust column names and types as needed to match your CSV files.

DROP TABLE IF EXISTS comments CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS stackoverflow_posts CASCADE;

CREATE TABLE stackoverflow_posts (
    id INTEGER PRIMARY KEY,
    post_type_id SMALLINT,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name VARCHAR,
    last_editor_user_id INTEGER,
    last_editor_display_name VARCHAR,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title VARCHAR,
    tags VARCHAR,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    closed_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license VARCHAR
);

CREATE TABLE comments (
    id INTEGER PRIMARY KEY,
    post_id INTEGER,
    score INTEGER,
    text TEXT,
    creation_date TIMESTAMP,
    user_display_name VARCHAR,
    user_id INTEGER,
    content_license VARCHAR
);

CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    reputation INTEGER,
    creation_date TIMESTAMP,
    display_name VARCHAR,
    last_access_date TIMESTAMP,
    website_url VARCHAR,
    location VARCHAR,
    about_me TEXT,
    views INTEGER,
    up_votes INTEGER,
    down_votes INTEGER,
    profile_image_url VARCHAR,
    email_hash VARCHAR,
    account_id INTEGER
);

-- StackOverflow dataset table definitions.
-- These tables mirror the StackOverflow public data dump schema.

-- Drop existing tables to start fresh.
DROP TABLE IF EXISTS badges CASCADE;
DROP TABLE IF EXISTS comments CASCADE;
DROP TABLE IF EXISTS post_history CASCADE;
DROP TABLE IF EXISTS post_links CASCADE;
DROP TABLE IF EXISTS posts_answers CASCADE;
DROP TABLE IF EXISTS posts_moderator_nomination CASCADE;
DROP TABLE IF EXISTS posts_orphaned_tag_wiki CASCADE;
DROP TABLE IF EXISTS posts_privilege_wiki CASCADE;
DROP TABLE IF EXISTS posts_questions CASCADE;
DROP TABLE IF EXISTS posts_tag_wiki CASCADE;
DROP TABLE IF EXISTS posts_tag_wiki_excerpt CASCADE;
DROP TABLE IF EXISTS posts_wiki_placeholder CASCADE;
DROP TABLE IF EXISTS stackoverflow_posts CASCADE;
DROP TABLE IF EXISTS tags CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS votes CASCADE;

CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    reputation INTEGER,
    creation_date TIMESTAMP,
    display_name TEXT,
    last_access_date TIMESTAMP,
    website_url TEXT,
    location TEXT,
    about_me TEXT,
    views INTEGER,
    up_votes INTEGER,
    down_votes INTEGER,
    account_id INTEGER
);

CREATE TABLE badges (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    name TEXT,
    date TIMESTAMP,
    class INTEGER,
    tag_based BOOLEAN
);

CREATE TABLE tags (
    id INTEGER PRIMARY KEY,
    tag_name TEXT,
    count INTEGER,
    excerpt_post_id INTEGER,
    wiki_post_id INTEGER
);

CREATE TABLE stackoverflow_posts (
    id INTEGER PRIMARY KEY,
    post_type_id INTEGER,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name TEXT,
    last_editor_user_id INTEGER,
    last_editor_display_name TEXT,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title TEXT,
    tags TEXT,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    close_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license TEXT
);

CREATE TABLE posts_questions (
    id INTEGER PRIMARY KEY,
    post_type_id INTEGER,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name TEXT,
    last_editor_user_id INTEGER,
    last_editor_display_name TEXT,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title TEXT,
    tags TEXT,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    close_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license TEXT
);

CREATE TABLE posts_answers (
    id INTEGER PRIMARY KEY,
    post_type_id INTEGER,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name TEXT,
    last_editor_user_id INTEGER,
    last_editor_display_name TEXT,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title TEXT,
    tags TEXT,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    close_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license TEXT
);

CREATE TABLE posts_moderator_nomination (
    id INTEGER PRIMARY KEY,
    post_type_id INTEGER,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name TEXT,
    last_editor_user_id INTEGER,
    last_editor_display_name TEXT,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title TEXT,
    tags TEXT,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    close_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license TEXT
);

CREATE TABLE posts_orphaned_tag_wiki (
    id INTEGER PRIMARY KEY,
    post_type_id INTEGER,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name TEXT,
    last_editor_user_id INTEGER,
    last_editor_display_name TEXT,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title TEXT,
    tags TEXT,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    close_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license TEXT
);

CREATE TABLE posts_privilege_wiki (
    id INTEGER PRIMARY KEY,
    post_type_id INTEGER,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name TEXT,
    last_editor_user_id INTEGER,
    last_editor_display_name TEXT,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title TEXT,
    tags TEXT,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    close_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license TEXT
);

CREATE TABLE posts_tag_wiki (
    id INTEGER PRIMARY KEY,
    post_type_id INTEGER,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name TEXT,
    last_editor_user_id INTEGER,
    last_editor_display_name TEXT,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title TEXT,
    tags TEXT,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    close_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license TEXT
);

CREATE TABLE posts_tag_wiki_excerpt (
    id INTEGER PRIMARY KEY,
    post_type_id INTEGER,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name TEXT,
    last_editor_user_id INTEGER,
    last_editor_display_name TEXT,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title TEXT,
    tags TEXT,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    close_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license TEXT
);

CREATE TABLE posts_wiki_placeholder (
    id INTEGER PRIMARY KEY,
    post_type_id INTEGER,
    accepted_answer_id INTEGER,
    parent_id INTEGER,
    creation_date TIMESTAMP,
    deletion_date TIMESTAMP,
    score INTEGER,
    view_count INTEGER,
    body TEXT,
    owner_user_id INTEGER,
    owner_display_name TEXT,
    last_editor_user_id INTEGER,
    last_editor_display_name TEXT,
    last_edit_date TIMESTAMP,
    last_activity_date TIMESTAMP,
    title TEXT,
    tags TEXT,
    answer_count INTEGER,
    comment_count INTEGER,
    favorite_count INTEGER,
    close_date TIMESTAMP,
    community_owned_date TIMESTAMP,
    content_license TEXT
);

CREATE TABLE comments (
    id INTEGER PRIMARY KEY,
    post_id INTEGER,
    score INTEGER,
    text TEXT,
    creation_date TIMESTAMP,
    user_id INTEGER,
    content_license TEXT
);

CREATE TABLE votes (
    id INTEGER PRIMARY KEY,
    post_id INTEGER,
    vote_type_id INTEGER,
    creation_date TIMESTAMP
);

CREATE TABLE post_history (
    id INTEGER PRIMARY KEY,
    post_history_type_id INTEGER,
    post_id INTEGER,
    revision_guid TEXT,
    creation_date TIMESTAMP,
    user_id INTEGER,
    text TEXT,
    content_license TEXT
);

CREATE TABLE post_links (
    id INTEGER PRIMARY KEY,
    creation_date TIMESTAMP,
    post_id INTEGER,
    related_post_id INTEGER,
    link_type_id INTEGER
);

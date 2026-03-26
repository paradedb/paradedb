-- Create BM25 indexes on the StackOverflow dataset.

CREATE INDEX stackoverflow_posts_idx ON stackoverflow_posts
USING bm25 (id, body, title, tags, score, view_count, creation_date, owner_user_id)
WITH (
    key_field = 'id',
    text_fields = '{"body": {"fast": true}, "title": {"fast": true}, "tags": {"fast": true}}'
);

CREATE INDEX posts_questions_idx ON posts_questions
USING bm25 (id, body, title, tags, score, view_count, creation_date, owner_user_id)
WITH (
    key_field = 'id',
    text_fields = '{"body": {"fast": true}, "title": {"fast": true}, "tags": {"fast": true}}'
);

CREATE INDEX posts_answers_idx ON posts_answers
USING bm25 (id, body, title, score, creation_date, owner_user_id, parent_id)
WITH (
    key_field = 'id',
    text_fields = '{"body": {"fast": true}, "title": {"fast": true}}'
);

CREATE INDEX comments_idx ON comments
USING bm25 (id, text, score, post_id, creation_date, user_id)
WITH (
    key_field = 'id',
    text_fields = '{"text": {"fast": true}}'
);

CREATE INDEX users_idx ON users
USING bm25 (id, display_name, about_me, reputation, location, creation_date)
WITH (
    key_field = 'id',
    text_fields = '{"display_name": {"fast": true}, "about_me": {"fast": true}, "location": {"fast": true}}'
);

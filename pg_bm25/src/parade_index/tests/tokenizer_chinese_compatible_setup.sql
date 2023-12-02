CREATE TABLE IF NOT EXISTS "source" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(255) NOT NULL,
    "created_at" TIMESTAMPTZ  DEFAULT now() NOT NULL,
    "updated_at" TIMESTAMPTZ  DEFAULT now() NOT NULL,
    "deleted_at" TIMESTAMPTZ,
    UNIQUE (name)
);

CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT,
    content JSONB,
    unix_timestamp_milli BIGINT,
    like_count INT,
    dislike_count INT,
    comment_count INT
);


INSERT INTO posts (author, title, message, content, unix_timestamp_milli, like_count, dislike_count, comment_count)
VALUES
    ('张伟', '第一篇文章', '这是第一篇文章的内容', '{"details": "这里有一些JSON内容"}', EXTRACT(EPOCH FROM now()) * 1000, 25, 1, 5),
    ('李娜', '第二篇文章', '这是第二篇文章的内容', '{"details": "这里有更多的JSON内容"}', EXTRACT(EPOCH FROM now()) * 1000, 75, 2, 10),
    ('王芳', '第三篇文章', '这是第三篇文章的信息', '{"details": "还有一些JSON内容"}', EXTRACT(EPOCH FROM now()) * 1000, 15, 0, 3);


CREATE INDEX idx_posts_fts
ON posts
USING bm25 ((posts.*))
WITH (
    key_field='id',
    text_fields='{
        author: {tokenizer: {type: "chinese_compatible"}, record: "position"},
        title: {tokenizer: {type: "chinese_compatible"}, record: "position"},
        message: {tokenizer: {type: "chinese_compatible"},record: "position"}
    }',
    json_fields='{
        content: {}
    }',
    numeric_fields='{
        unix_timestamp_milli: {},
        like_count: {},
        dislike_count: {},
        comment_count: {}
    }'
);


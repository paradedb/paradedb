CREATE TABLE IF NOT EXISTS t (
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

INSERT INTO t (author, title, message, content, unix_timestamp_milli, like_count, dislike_count, comment_count)
VALUES
    ('김민준', '첫 번째 기사', '이것은 첫 번째 기사의 내용입니다', '{"details": "여기에는 일부 JSON 내용이 있습니다"}', EXTRACT(EPOCH FROM now()) * 1000, 25, 1, 5),
    ('이하은', '두 번째 기사', '이것은 두 번째 기사의 내용입니다', '{"details": "여기에는 더 많은 JSON 내용이 있습니다"}', EXTRACT(EPOCH FROM now()) * 1000, 75, 2, 10),
    ('박지후', '세 번째 기사', '이것은 세 번째 기사의 정보입니다', '{"details": "여기에도 일부 JSON 내용이 있습니다"}', EXTRACT(EPOCH FROM now()) * 1000, 15, 0, 3);

CREATE INDEX idx_t
ON t
USING bm25 ((t.*))
WITH (
    text_fields='{
        author: {tokenizer: {type: "lindera"}, record: "position"},
        title: {tokenizer: {type: "lindera"}, record: "position"},
        message: {tokenizer: {type: "lindera"}, record: "position"}
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

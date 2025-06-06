CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS issue_2585_test;

CREATE TABLE issue_2585_test (
    id SERIAL PRIMARY KEY,
    content TEXT,
    titles TEXT[],
    metadata JSONB,
    is_null BOOLEAN
);

INSERT INTO issue_2585_test (content, titles, metadata, is_null) VALUES
    ('Sample content 1', ARRAY['Title 1', 'Title 2'], '{"key": "value1"}', false),
    (NULL, NULL, NULL, true),
    ('Another content', ARRAY['Title 3'], '{"key": "value2"}', false),
    ('Content with null titles', NULL, '{"key": "value3"}', false),
    (NULL, ARRAY['Title 4', 'Title 5'], NULL, false),
    ('Content with null metadata', ARRAY['Title 6'], NULL, false),
    ('All fields present', NULL, NULL, false),
    (NULL, NULL, NULL, true);

CREATE INDEX ON issue_2585_test USING bm25 (
    id,
    content,
    titles,
    metadata,
    is_null
) WITH (
    key_field = 'id'
);

SELECT * FROM issue_2585_test WHERE content @@@ 'content' AND titles IS NOT NULL AND metadata IS NOT NULL AND is_null IS NOT NULL;
SELECT * FROM issue_2585_test WHERE content @@@ 'fields' AND titles IS NULL AND metadata IS NULL AND is_null IS FALSE;

DROP TABLE IF EXISTS issue_2585_test;

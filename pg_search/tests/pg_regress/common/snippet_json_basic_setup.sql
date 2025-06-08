CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS snippet_test;
CREATE TABLE snippet_test (
    id SERIAL PRIMARY KEY,
    content TEXT,
    metadata_json JSON,
    metadata_jsonb JSONB
);

INSERT INTO snippet_test (content, metadata_json, metadata_jsonb) VALUES
('This is a test test of the snippet function with multiple test words',
 '{"name": "test1", "tags": ["test", "snippet"], "metadata": {"created": "2023-01-01", "priority": 1}}',
 '{"id": 1, "details": {"author": {"first_name": "John", "last_name": "Doe", "description": "A test author"}, "stats": {"views": 100, "likes": 50}}, "active": true}'
),
('Another test of the snippet snippet function with repeated snippet words',
 '{"name": "test2", "scores": [10, 20, 30], "config": {"enabled": true, "settings": {"mode": "advanced", "limit": 5}}}',
 '{"id": 2, "nested": {"level1": {"level2": {"level3": "deep value"}}, "array": [1, "two", 3.14]}, "status": "active"}'
),
('Yet another test test test of the function function function',
 '{"name": "test3", "mixed": [{"key": "value"}, 42, null, true], "timestamp": "2023-12-31T23:59:59Z"}',
 '{"id": 3, "data": {"numbers": [1.1, 2.2, 3.3], "flags": {"debug": true, "test": false}}, "tags": ["alpha", "beta"]}'
),
('test Lorem ipsum dolor sit amet...test',
 '{"name": "test4", "complex": {"arrays": [[1,2], [3,4]], "object": {"null": null, "bool": false, "num": 3.14159}}}',
 '{"id": 4, "metadata": {"created_at": "2023-12-01", "updated_at": "2023-12-31", "versions": [1, 2, 3]}, "settings": {"notifications": {"email": true, "push": false}, "theme": "dark"}}'
);

CREATE INDEX ON snippet_test USING bm25 (
    id,
    metadata_json,
    metadata_jsonb
) WITH (
    key_field = 'id'
);

CREATE INDEX pages_index ON pages
USING bm25 ("id",
    "content",
    "title",
    "parents",
    "fileId",
    "sizeInBytes",
    "createdAt"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "fileId": {
            "tokenizer": {"type": "keyword"}, "fast": true
        },
        "content": {
            "tokenizer": {"type": "icu"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "icu"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "icu"}, "fast": true
        }
    }'
);

CREATE INDEX files_index ON files
USING bm25 ("id",
    "content",
    "documentId",
    "title",
    "parents",
    "sizeInBytes",
    "createdAt"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "documentId": {
            "tokenizer": {"type": "keyword"}, "fast": true
        },
        "content": {
            "tokenizer": {"type": "icu"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "icu"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "icu"}, "fast": true
        }
    }'
);

CREATE INDEX documents_index ON documents
USING bm25 ("id",
    "content",
    "title",
    "parents",
    "createdAt"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "content": {
            "tokenizer": {"type": "icu"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "icu"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "icu"}, "fast": true
        }
    }'
);

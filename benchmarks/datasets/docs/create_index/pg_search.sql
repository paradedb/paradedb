CREATE INDEX pages_index ON pages
USING bm25 (
    "id",
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
            "tokenizer": {"type": "default"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }',
    target_segment_count = 48
);

CREATE INDEX files_index ON files
USING bm25 (
    "id",
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
            "tokenizer": {"type": "default"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }',
    target_segment_count = 48
);

CREATE INDEX documents_index ON documents
USING bm25 (
    "id",
    "content",
    "title",
    "parents",
    "createdAt"
)
WITH (
    key_field = 'id',
    text_fields = '{
        "content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "parents": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }',
    target_segment_count = 48
);

CREATE INDEX documents_inner_join_files_inner_join_pages_bm25 ON documents_inner_join_files_inner_join_pages
USING bm25 (row_id, doc_parents, doc_title, file_title, file_content, page_content, page_title, page_size_in_bytes)
WITH (
    key_field = 'row_id',
    text_fields = '{
        "doc_parents": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "doc_title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "file_title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "file_content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "page_content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "page_title": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }'
);

CREATE INDEX files_inner_join_documents_bm25 ON files_inner_join_documents
USING bm25 (row_id, file_title, file_content, doc_parents, doc_title, file_created_at)
WITH (
    key_field = 'row_id',
    text_fields = '{
        "file_title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "file_content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "doc_parents": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "doc_title": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }'
);

CREATE INDEX files_left_join_documents_bm25 ON files_left_join_documents
USING bm25 (row_id, file_id, file_title, doc_title, doc_parents)
WITH (
  key_field = 'row_id',
  text_fields = '{
    "file_id":      {"tokenizer": {"type": "keyword"}, "fast": true},
    "file_title":   {"tokenizer": {"type": "default"}, "fast": true},
    "doc_title":    {"tokenizer": {"type": "default"}, "fast": true},
    "doc_parents":  {"tokenizer": {"type": "default"}, "fast": true}
  }'
);

CREATE INDEX files_inner_join_pages_bm25 ON files_inner_join_pages
USING bm25 (row_id, file_content, file_title, page_content, page_title)
WITH (
    key_field = 'row_id',
    text_fields = '{
        "file_content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "file_title": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "page_content": {
            "tokenizer": {"type": "default"}, "fast": true
        },
        "page_title": {
            "tokenizer": {"type": "default"}, "fast": true
        }
    }'
);

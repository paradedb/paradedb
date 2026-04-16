CREATE INDEX pages_index ON pages
USING bm25 (
    "id",
    ("content"::pdb.unicode_words('columnar=true')),
    ("title"::pdb.unicode_words('columnar=true')),
    ("parents"::pdb.unicode_words('columnar=true')),
    ("fileId"::pdb.literal),
    "sizeInBytes",
    "createdAt"
)
WITH (
    key_field = 'id',
    sort_by = 'fileId ASC NULLS FIRST'
);

CREATE INDEX files_index ON files
USING bm25 (
    "id",
    ("content"::pdb.unicode_words('columnar=true')),
    ("documentId"::pdb.literal),
    ("title"::pdb.unicode_words('columnar=true')),
    ("parents"::pdb.unicode_words('columnar=true')),
    "sizeInBytes",
    "createdAt"
)
WITH (
    key_field = 'id',
    sort_by = 'documentId ASC NULLS FIRST'
);

CREATE INDEX documents_index ON documents
USING bm25 (
    "id",
    ("content"::pdb.unicode_words('columnar=true')),
    ("title"::pdb.unicode_words('columnar=true')),
    ("parents"::pdb.unicode_words('columnar=true')),
    "createdAt"
)
WITH (
    key_field = 'id',
    sort_by = 'id ASC NULLS FIRST'
);

\i common/common_setup.sql

CREATE TABLE mlt (
    id SERIAL PRIMARY KEY,
    text_field_a TEXT,
    text_field_b TEXT,
    numeric_field INTEGER,
    json_field JSONB
);

INSERT INTO mlt (text_field_a, text_field_b, json_field, numeric_field) VALUES
    ('aaa bbb ccc', 'foo bar', '{"color": "aaa bbb ccc"}', 1),
    ('aaa aaa', 'baz baz', '{"color": "aaa aaa"}', 1),
    ('ddd eee fff', 'foo foo foo', '{"color": "ddd eee fff"}', 2),
    ('aaa aaa', 'baz baz', '{"color": "aaa aaa"}', 3);

CREATE INDEX ON mlt USING bm25 (id, text_field_a, text_field_b, json_field, numeric_field) WITH (key_field = 'id');

SELECT * from mlt where id @@@ pdb.more_like_this(1);
SELECT * FROM mlt where id @@@ pdb.more_like_this(1, ARRAY['text_field_a']);
SELECT * FROM mlt where id @@@ pdb.more_like_this(1, ARRAY['text_field_b']);
SELECT * FROM mlt where id @@@ pdb.more_like_this(1, ARRAY['numeric_field']);

-- Term must appear n times in the source doc to be considered
SELECT * FROM mlt where id @@@ pdb.more_like_this(2, min_term_frequency => 2);
SELECT * FROM mlt where id @@@ pdb.more_like_this(2, min_term_frequency => 3);

-- Term must appear in at least n docs to be considered
SELECT * from mlt where id @@@ pdb.more_like_this(1, min_doc_frequency => 2);
SELECT * from mlt where id @@@ pdb.more_like_this(1, min_doc_frequency => 3);

-- Term must appear in at most n docs to be considered
SELECT * from mlt where id @@@ pdb.more_like_this(1, ARRAY['text_field_a'], max_doc_frequency => 2);
SELECT * from mlt where id @@@ pdb.more_like_this(1, ARRAY['text_field_a'], max_doc_frequency => 3);

-- Max term length
SELECT * from mlt where id @@@ pdb.more_like_this(1, ARRAY['text_field_a'], max_word_length => 2);
SELECT * from mlt where id @@@ pdb.more_like_this(1, ARRAY['text_field_a'], max_word_length => 3);

-- Stopwords
SELECT * from mlt where id @@@ pdb.more_like_this(1, ARRAY['text_field_a'], stopwords => ARRAY['aaa']);

-- Max query terms
SELECT * from mlt where id @@@ pdb.more_like_this(1, ARRAY['text_field_a'], max_query_terms => 2);

-- JSON not supported
SELECT * FROM mlt where id @@@ pdb.more_like_this(1, ARRAY['json_field']);
-- Document ID doesn't exist
SELECT * FROM mlt where id @@@ pdb.more_like_this(100);

DROP TABLE mlt;

\i common/common_setup.sql

SELECT pg_typeof(pdb.more_like_this(1)) AS value_type,
       pg_typeof(pdb.more_like_this(document => '{"body":"alpha"}')) AS document_type;

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

CREATE INDEX ON mlt USING paradedb (id, text_field_a, text_field_b, json_field, numeric_field) WITH (key_field = 'id');

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

-- The LHS selects the source row, independently of the configured key.
SELECT ARRAY(SELECT id FROM mlt WHERE id @@@ pdb.more_like_this(3, ARRAY['text_field_a']) ORDER BY id)
     = ARRAY(SELECT id FROM mlt WHERE numeric_field @@@ pdb.more_like_this(
         2, fields => ARRAY['text_field_a']) ORDER BY id)
       AS explicit_lookup_matches_key;

SELECT pdb.more_like_this(1, '{text_field_a}')::text::jsonb
     = pdb.more_like_this(1, ARRAY['text_field_a'])::text::jsonb AS legacy_fields_literal;

DROP TABLE mlt;

-- Field-less more_like_this skips vector columns (issue #5826)
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE mlt_vec (
    id SERIAL PRIMARY KEY,
    description TEXT,
    embedding vector(3)
);

INSERT INTO mlt_vec (description, embedding) VALUES
    ('aaa bbb ccc', '[1,2,3]'),
    ('aaa aaa', '[4,5,6]'),
    ('ddd eee fff', '[7,8,9]');

CREATE INDEX ON mlt_vec USING paradedb (id, description, embedding) WITH (key_field = 'id');

SELECT id, description FROM mlt_vec WHERE id @@@ pdb.more_like_this(1);
SELECT id, description FROM mlt_vec WHERE id @@@ pdb.more_like_this(1, ARRAY['description']);
-- Vector not supported
SELECT id FROM mlt_vec WHERE id @@@ pdb.more_like_this(1, ARRAY['embedding']);

DROP TABLE mlt_vec;

CREATE TABLE mlt_keyless (
    tenant int,
    id int,
    body text,
    "Lookup Code" text,
    PRIMARY KEY (tenant, id)
);
INSERT INTO mlt_keyless VALUES
    (10, 1, 'alpha alpha', 'alpha''s'),
    (20, 1, 'beta beta', 'beta'),
    (10, 2, 'alpha alpha', NULL),
    (20, 2, 'beta beta', NULL);
CREATE INDEX mlt_keyless_idx ON mlt_keyless USING paradedb (tenant, id, body, "Lookup Code");

-- A non-first, quoted LHS supplies the lookup column, not the rewritten planner anchor.
SELECT tenant, id FROM mlt_keyless t
WHERE "Lookup Code" @@@ pdb.more_like_this('alpha''s'::text, fields => ARRAY['body'])
ORDER BY tenant, id;

-- Either source is valid for a duplicated value, but their content must not be combined.
SELECT ARRAY(SELECT tenant FROM mlt_keyless t WHERE id @@@ pdb.more_like_this(
    1, fields => ARRAY['body']) ORDER BY tenant)
    IN (ARRAY[10, 10], ARRAY[20, 20]) AS one_source;

SELECT tenant, id FROM mlt_keyless t
WHERE t @@@ pdb.more_like_this(document => '{"body": "alpha alpha"}')
ORDER BY tenant, id;

SELECT tenant, id FROM mlt_keyless t WHERE id @@@ pdb.more_like_this(999, fields => ARRAY['body']);

SET plan_cache_mode = force_generic_plan;
PREPARE mlt_lookup(text) AS
SELECT tenant, id FROM mlt_keyless t WHERE "Lookup Code" @@@ pdb.more_like_this(
    $1, fields => ARRAY['body'])
ORDER BY tenant, id;
EXECUTE mlt_lookup('alpha''s');
EXECUTE mlt_lookup('beta');

SET paradedb.enable_custom_scan = off;
SET enable_indexscan = off;
SET enable_bitmapscan = off;
DISCARD PLANS;
EXECUTE mlt_lookup('alpha''s');
EXECUTE mlt_lookup('beta');
DEALLOCATE mlt_lookup;
RESET plan_cache_mode;
RESET paradedb.enable_custom_scan;
RESET enable_indexscan;
RESET enable_bitmapscan;

SELECT tenant, id FROM mlt_keyless t WHERE t @@@ pdb.more_like_this(key_value => 1);

SELECT tenant, id FROM mlt_keyless WHERE "Lookup Code" @@@
    pdb.more_like_this('alpha''s'::text, fields => ARRAY['body'])::pdb.boost(2)
ORDER BY tenant, id;

SET plan_cache_mode = force_generic_plan;
PREPARE mlt_query(pdb.query) AS
SELECT tenant, id FROM mlt_keyless WHERE "Lookup Code" @@@ $1 ORDER BY tenant, id;
EXECUTE mlt_query(pdb.more_like_this('alpha''s'::text, fields => ARRAY['body']));
DEALLOCATE mlt_query;
RESET plan_cache_mode;

-- An aliased tokenizer cast still identifies its underlying heap column.
DROP INDEX mlt_keyless_idx;
CREATE INDEX mlt_keyless_idx ON mlt_keyless USING paradedb (
    tenant, id, body, ("Lookup Code"::pdb.literal('alias=lookup_code')));
SELECT tenant, id FROM mlt_keyless WHERE
    ("Lookup Code"::pdb.literal('alias=lookup_code')) @@@
    pdb.more_like_this('alpha''s'::text, fields => ARRAY['body'])
ORDER BY tenant, id;

-- Computed expressions are not silently interpreted as their underlying heap column.
DROP INDEX mlt_keyless_idx;
CREATE INDEX mlt_keyless_idx ON mlt_keyless USING paradedb (
    tenant, id, body, (lower("Lookup Code")::pdb.literal('alias=lookup_lower')));
SELECT tenant, id FROM mlt_keyless WHERE lower("Lookup Code") @@@
    pdb.more_like_this('alpha''s'::text, fields => ARRAY['body']);

DROP TABLE mlt_keyless;

-- The legacy key-value overload keeps working for non-integer keys too.
CREATE TABLE mlt_text_key (id text PRIMARY KEY, body text);
INSERT INTO mlt_text_key VALUES ('alpha', 'alpha alpha'), ('beta', 'beta beta');
CREATE INDEX mlt_text_key_idx ON mlt_text_key USING paradedb (id, body) WITH (key_field = 'id');
SELECT id FROM mlt_text_key WHERE id @@@ pdb.more_like_this(
    key_value => 'alpha'::text, fields => ARRAY['body']) ORDER BY id;
DROP TABLE mlt_text_key;

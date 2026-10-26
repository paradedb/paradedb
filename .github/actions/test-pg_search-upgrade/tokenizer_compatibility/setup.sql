-- This file runs against the prior extension release. Keep its SQL compatible
-- with every starting release in the upgrade matrix.
CREATE TABLE tokenizer_compatibility (
    id INTEGER PRIMARY KEY,
    about_me TEXT,
    bio TEXT,
    legacy_chinese TEXT,
    legacy_japanese TEXT,
    legacy_korean TEXT,
    current_chinese TEXT,
    current_japanese TEXT,
    current_korean TEXT,
    raw_text TEXT,
    autocomplete TEXT
);

INSERT INTO tokenizer_compatibility VALUES (
    1,
    'Hello World 🚀',
    'Hello World 🚀',
    'hello world',
    'hello world',
    'hello world',
    'hello world',
    'hello world',
    'hello world',
    'compat',
    'hello world'
);

CREATE INDEX tokenizer_compatibility_idx ON tokenizer_compatibility
USING paradedb (
    id,
    (about_me::pdb.unicode_words('remove_emojis=false')),
    (bio::pdb.unicode_words('lowercase=false')),
    legacy_chinese,
    legacy_japanese,
    legacy_korean,
    (current_chinese::pdb.lindera(chinese, 'alias=current_chinese')),
    (current_japanese::pdb.lindera(japanese, 'alias=current_japanese')),
    (current_korean::pdb.lindera(korean, 'alias=current_korean')),
    raw_text,
    (autocomplete::pdb.ngram(1, 3, 'prefix_only=true'))
)
WITH (
    key_field = 'id',
    search_tokenizer = 'unicode_words',
    text_fields = '{
        "legacy_chinese": {"tokenizer": {"type": "chinese_lindera"}},
        "legacy_japanese": {"tokenizer": {"type": "japanese_lindera"}},
        "legacy_korean": {"tokenizer": {"type": "korean_lindera"}},
        "raw_text": {"tokenizer": {"type": "raw"}}
    }'
);

CREATE VIEW tokenizer_compatibility_results AS
SELECT 'unicode_field' AS query, ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE about_me ||| 'hello' ORDER BY id
) AS ids
UNION ALL
SELECT 'unicode_query_cast', ARRAY(
    SELECT id FROM tokenizer_compatibility
    WHERE about_me ||| 'hello'::pdb.unicode_words('remove_emojis=false') ORDER BY id
)
UNION ALL
SELECT 'unicode_lowercase_false', ARRAY(
    SELECT id FROM tokenizer_compatibility
    WHERE bio ||| 'Hello'::pdb.unicode_words('lowercase=false') ORDER BY id
)
UNION ALL
SELECT 'legacy_chinese', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE legacy_chinese ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'legacy_japanese', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE legacy_japanese ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'legacy_korean', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE legacy_korean ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'current_chinese', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE current_chinese ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'current_japanese', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE current_japanese ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'current_korean', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE current_korean ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'raw', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE raw_text ||| 'compat' ORDER BY id
)
UNION ALL
SELECT 'index_search_tokenizer', ARRAY(
    SELECT id FROM tokenizer_compatibility WHERE autocomplete ||| 'hel' ORDER BY id
);

CREATE TABLE tokenizer_compatibility_results_before_upgrade AS
SELECT * FROM tokenizer_compatibility_results;

SET plpgsql.check_asserts = on;
DO $$
BEGIN
    ASSERT (
        SELECT count(*) FROM tokenizer_compatibility_results_before_upgrade
        WHERE ids = ARRAY[1]
    ) = 11, 'every tokenizer compatibility case must match the original row before upgrade';
END;
$$;

CREATE TABLE tokenizer_compatibility_schema_before_upgrade AS
SELECT 'tokenizer_compatibility_idx' AS index_name, name, tokenizer
FROM paradedb.schema('tokenizer_compatibility_idx');

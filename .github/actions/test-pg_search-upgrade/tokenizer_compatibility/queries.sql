SET plpgsql.check_asserts = on;
SET paradedb.global_mutable_segment_rows = 1000;

INSERT INTO tokenizer_compatibility (
    id, about_me, bio, legacy_chinese, legacy_japanese, legacy_korean,
    current_chinese, current_japanese, current_korean, raw_text
) VALUES (
    2,
    'Hello Again 🚀',
    'Hello Again 🚀',
    '北京的新餐馆',
    '東京の新しいカフェ',
    '김민준',
    '北京的新餐馆',
    '東京の新しいカフェ',
    '김민준',
    'compat'
);

INSERT INTO tokenizer_search_tokenizer_compatibility VALUES (2, 'hello again');
INSERT INTO tokenizer_filter_compatibility (id) VALUES (2);
INSERT INTO tokenizer_option_matrix (id) VALUES (2);

DO $$
BEGIN
    ASSERT EXISTS (
        SELECT 1 FROM paradedb.index_info('tokenizer_compatibility_idx') WHERE mutable
    ), 'the upgraded index must have a mutable segment';
    ASSERT EXISTS (
        SELECT 1 FROM paradedb.index_info('tokenizer_search_tokenizer_idx') WHERE mutable
    ), 'the search_tokenizer index must have a mutable segment';
    ASSERT EXISTS (
        SELECT 1 FROM paradedb.index_info('tokenizer_filter_compatibility_idx') WHERE mutable
    ), 'the filter index must have a mutable segment';
    ASSERT EXISTS (
        SELECT 1 FROM paradedb.index_info('tokenizer_option_matrix_idx') WHERE mutable
    ), 'the tokenizer option matrix must have a mutable segment';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_option_matrix_results current
        FULL JOIN tokenizer_option_matrix_results_before_upgrade old USING (query, expected_match)
        WHERE current.ids IS DISTINCT FROM
            CASE WHEN old.ids = ARRAY[1] THEN ARRAY[1, 2] ELSE old.ids END
    ), 'tokenizer options and boundary behavior must survive mutable writes';
    ASSERT NOT EXISTS (
        SELECT 1 FROM (
            SELECT name, tokenizer FROM paradedb.schema('tokenizer_option_matrix_idx')
            WHERE name NOT IN ('id', 'ctid')
        ) current
        FULL JOIN tokenizer_option_matrix_schema_before_upgrade old USING (name)
        WHERE current.name IS NULL OR old.name IS NULL OR current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'tokenizer option names must remain unchanged after upgrade';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_filter_results current
        FULL JOIN tokenizer_filter_results_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM
            CASE WHEN old.ids = ARRAY[1] THEN ARRAY[1, 2] ELSE old.ids END
    ), 'filter queries must keep their results after mutable writes';
    ASSERT NOT EXISTS (
        SELECT 1 FROM paradedb.schema('tokenizer_filter_compatibility_idx') current
        FULL JOIN tokenizer_filter_schema_before_upgrade old USING (name)
        WHERE current.name IS NULL OR old.name IS NULL OR current.tokenizer IS DISTINCT FROM old.tokenizer
            OR current.normalizer IS DISTINCT FROM old.normalizer
    ), 'filter tokenizer names must remain unchanged';
    ASSERT (SELECT raw_result #>> '{buckets,0,key}' = 'MiXeD'
                   AND lower_result #>> '{buckets,0,key}' = 'mixed'
                   AND (raw_result #>> '{buckets,0,doc_count}')::integer = 2
                   AND (lower_result #>> '{buckets,0,doc_count}')::integer = 2
            FROM tokenizer_normalizer_results),
        'normalizers must preserve fast-field values after mutable writes';
    ASSERT NOT EXISTS (
        SELECT 1
        FROM tokenizer_compatibility_results current
        FULL JOIN tokenizer_compatibility_results_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM old.ids || ARRAY[2]
    ), 'every tokenizer must keep matching old rows and the new mutable row';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_compatibility_negative_results WHERE ids <> ARRAY[]::integer[]
    ), 'negative tokenizer queries must not match after mutable writes';
    ASSERT (SELECT ARRAY(
        SELECT id FROM tokenizer_search_tokenizer_compatibility
        WHERE autocomplete ||| 'hel' ORDER BY id
    )) = ARRAY[1, 2], 'the upgraded search_tokenizer index must match the old and new mutable rows';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_search_tokenizer_compatibility
        WHERE autocomplete ||| 'hello'
    ), 'the upgraded search_tokenizer override must not be ignored';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_compatibility WHERE bio ||| 'hello'
    ), 'lowercase=false must keep lowercase input distinct after upgrade';
    ASSERT NOT EXISTS (
        SELECT 1
        FROM (
            SELECT 'tokenizer_compatibility_idx' AS index_name, name, tokenizer
            FROM paradedb.schema('tokenizer_compatibility_idx')
        ) current
        FULL JOIN tokenizer_compatibility_schema_before_upgrade old USING (index_name, name)
        WHERE current.index_name IS NULL
            OR old.index_name IS NULL
            OR current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'the upgrade must preserve all stored tokenizer names without REINDEX';
END;
$$;

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO tokenizer_compatibility (
    id, about_me, bio, legacy_chinese, legacy_japanese, legacy_korean,
    current_chinese, current_japanese, current_korean, raw_text
) VALUES (
    3,
    'Hello There 🚀',
    'Hello There 🚀',
    '北京的新餐馆',
    '東京の新しいカフェ',
    '김민준',
    '北京的新餐馆',
    '東京の新しいカフェ',
    '김민준',
    'compat'
);

INSERT INTO tokenizer_search_tokenizer_compatibility VALUES (3, 'hello there');
INSERT INTO tokenizer_filter_compatibility (id) VALUES (3);
INSERT INTO tokenizer_option_matrix (id) VALUES (3);

DO $$
BEGIN
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_filter_results current
        FULL JOIN tokenizer_filter_results_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM
            CASE WHEN old.ids = ARRAY[1] THEN ARRAY[1, 2, 3] ELSE old.ids END
    ), 'filter queries must keep their results after immutable writes';
    ASSERT NOT EXISTS (
        SELECT 1 FROM paradedb.schema('tokenizer_filter_compatibility_idx') current
        FULL JOIN tokenizer_filter_schema_before_upgrade old USING (name)
        WHERE current.name IS NULL OR old.name IS NULL OR current.tokenizer IS DISTINCT FROM old.tokenizer
            OR current.normalizer IS DISTINCT FROM old.normalizer
    ), 'filter tokenizer names must remain unchanged after immutable writes';
    ASSERT (SELECT raw_result #>> '{buckets,0,key}' = 'MiXeD'
                   AND lower_result #>> '{buckets,0,key}' = 'mixed'
                   AND (raw_result #>> '{buckets,0,doc_count}')::integer = 3
                   AND (lower_result #>> '{buckets,0,doc_count}')::integer = 3
            FROM tokenizer_normalizer_results),
        'normalizers must preserve fast-field values after immutable writes';
    ASSERT NOT EXISTS (
        SELECT 1
        FROM tokenizer_compatibility_results current
        FULL JOIN tokenizer_compatibility_results_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM old.ids || ARRAY[2, 3]
    ), 'every tokenizer must keep matching old rows and new immutable rows';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_option_matrix_results current
        FULL JOIN tokenizer_option_matrix_results_before_upgrade old USING (query, expected_match)
        WHERE current.ids IS DISTINCT FROM
            CASE WHEN old.ids = ARRAY[1] THEN ARRAY[1, 2, 3] ELSE old.ids END
    ), 'tokenizer options and boundary behavior must survive immutable writes';
    ASSERT NOT EXISTS (
        SELECT 1 FROM (
            SELECT name, tokenizer FROM paradedb.schema('tokenizer_option_matrix_idx')
            WHERE name NOT IN ('id', 'ctid')
        ) current
        FULL JOIN tokenizer_option_matrix_schema_before_upgrade old USING (name)
        WHERE current.name IS NULL OR old.name IS NULL OR current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'immutable writes must retain tokenizer option names';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_compatibility_negative_results WHERE ids <> ARRAY[]::integer[]
    ), 'negative tokenizer queries must not match after immutable writes';
    ASSERT (SELECT ARRAY(
        SELECT id FROM tokenizer_search_tokenizer_compatibility
        WHERE autocomplete ||| 'hel' ORDER BY id
    )) = ARRAY[1, 2, 3], 'the upgraded search_tokenizer index must match old, mutable, and immutable rows';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_search_tokenizer_compatibility
        WHERE autocomplete ||| 'hello'
    ), 'immutable search_tokenizer writes must keep rejecting the longer query';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_compatibility WHERE bio ||| 'hello'
    ), 'lowercase=false must keep lowercase input distinct after immutable writes';
    ASSERT NOT EXISTS (
        SELECT 1
        FROM (
            SELECT 'tokenizer_compatibility_idx' AS index_name, name, tokenizer
            FROM paradedb.schema('tokenizer_compatibility_idx')
        ) current
        FULL JOIN tokenizer_compatibility_schema_before_upgrade old USING (index_name, name)
        WHERE current.index_name IS NULL
            OR old.index_name IS NULL
            OR current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'immutable writes must not change the stored tokenizer names';
END;
$$;

RESET paradedb.global_mutable_segment_rows;

-- A trimmed search override must not claim the legacy name of an untrimmed
-- field tokenizer when the reader registers tokenizers for a new index.
CREATE TABLE tokenizer_search_override_trim_compatibility (
    id INTEGER PRIMARY KEY,
    body TEXT DEFAULT '아름다운 우리나라'
);
INSERT INTO tokenizer_search_override_trim_compatibility (id) VALUES (1);
CREATE INDEX tokenizer_search_override_trim_compatibility_idx
ON tokenizer_search_override_trim_compatibility USING paradedb (
    id,
    (body::pdb.lindera(korean, 'keep_whitespace=true'))
)
WITH (search_tokenizer = 'lindera(language=korean,keep_whitespace=true,trim=true)');

DO $$
BEGIN
    ASSERT (SELECT ARRAY(
        SELECT id FROM tokenizer_search_override_trim_compatibility
        WHERE id @@@ paradedb.term('body', ' ') ORDER BY id
    )) = ARRAY[1],
        'a search_tokenizer override must not replace an untrimmed field tokenizer';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_search_override_trim_compatibility WHERE body ||| ' '
    ), 'the trimmed search_tokenizer override must still be used for uncast queries';
    ASSERT (SELECT tokenizer = 'korean_lindera_keepwhitespace:true'
            FROM paradedb.schema('tokenizer_search_override_trim_compatibility_idx')
            WHERE name = 'body'),
        'the field schema must retain its untrimmed tokenizer name';
END;
$$;

-- Verify that trim=true and trim=false get distinct registrations in a new index,
-- while the index built before upgrade still resolves its legacy tokenizer name.
CREATE TABLE tokenizer_trim_name_compatibility (
    id INTEGER PRIMARY KEY,
    trimmed TEXT DEFAULT ' hello ',
    untrimmed TEXT DEFAULT ' hello '
);
INSERT INTO tokenizer_trim_name_compatibility (id) VALUES (1);
CREATE INDEX tokenizer_trim_name_compatibility_idx ON tokenizer_trim_name_compatibility
USING paradedb (
    id,
    (trimmed::pdb.jieba('trim=true')),
    (untrimmed::pdb.jieba('trim=false'))
);

CREATE VIEW tokenizer_trim_name_results AS
SELECT 'trimmed_whitespace' AS query, ARRAY(
    SELECT id FROM tokenizer_trim_name_compatibility
    WHERE id @@@ paradedb.term('trimmed', ' ') ORDER BY id
) AS ids
UNION ALL
SELECT 'untrimmed_whitespace', ARRAY(
    SELECT id FROM tokenizer_trim_name_compatibility
    WHERE id @@@ paradedb.term('untrimmed', ' ') ORDER BY id
)
UNION ALL
SELECT 'trimmed_text', ARRAY(
    SELECT id FROM tokenizer_trim_name_compatibility
    WHERE trimmed ||| 'hello' ORDER BY id
)
UNION ALL
SELECT 'untrimmed_text', ARRAY(
    SELECT id FROM tokenizer_trim_name_compatibility
    WHERE untrimmed ||| 'hello' ORDER BY id
);

SET plpgsql.check_asserts = on;
DO $$
BEGIN
    ASSERT (SELECT count(*) FROM tokenizer_trim_name_results WHERE ids = ARRAY[1]) = 3,
        'the trimmed and untrimmed Jieba analyzers must both be registered after upgrade';
    ASSERT (SELECT count(*) FROM tokenizer_trim_name_results WHERE ids = ARRAY[]::integer[]) = 1,
        'trim=true must remove whitespace-only tokens after upgrade';
    ASSERT (SELECT count(*) FROM paradedb.schema('tokenizer_trim_name_compatibility_idx')
            WHERE tokenizer IN ('jieba[trim=true]', 'jieba')) = 2,
        'trim=true and trim=false must persist distinct tokenizer names';
END;
$$;

SET paradedb.global_mutable_segment_rows = 1000;
INSERT INTO tokenizer_trim_name_compatibility (id) VALUES (2);
DO $$
BEGIN
    ASSERT (SELECT count(*) FROM tokenizer_trim_name_results WHERE ids = ARRAY[1, 2]) = 3,
        'mutable writes must use the field-specific trim setting';
END;
$$;

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO tokenizer_trim_name_compatibility (id) VALUES (3);
DO $$
BEGIN
    ASSERT (SELECT count(*) FROM tokenizer_trim_name_results WHERE ids = ARRAY[1, 2, 3]) = 3,
        'immutable writes must use the field-specific trim setting';
END;
$$;
RESET paradedb.global_mutable_segment_rows;

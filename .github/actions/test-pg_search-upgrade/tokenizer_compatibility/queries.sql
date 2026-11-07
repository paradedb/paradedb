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

SET plpgsql.check_asserts = on;
SET paradedb.global_mutable_segment_rows = 1000;

INSERT INTO tokenizer_compatibility VALUES (
    2,
    'Hello Again 🚀',
    'Hello Again 🚀',
    'hello again',
    'hello again',
    'hello again',
    'hello again',
    'hello again',
    'hello again',
    'compat',
    'hello again'
);

DO $$
BEGIN
    ASSERT EXISTS (
        SELECT 1 FROM paradedb.index_info('tokenizer_compatibility_idx') WHERE mutable
    ), 'the upgraded index must have a mutable segment';
    ASSERT NOT EXISTS (
        SELECT 1
        FROM tokenizer_compatibility_results current
        FULL JOIN tokenizer_compatibility_results_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM old.ids || ARRAY[2]
    ), 'every tokenizer must keep matching old rows and the new mutable row';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_compatibility WHERE bio ||| 'hello'
    ), 'lowercase=false must keep lowercase input distinct after upgrade';
    ASSERT NOT EXISTS (
        SELECT 1
        FROM (
            SELECT 'tokenizer_compatibility_idx' AS index_name, name, tokenizer
            FROM paradedb.schema('tokenizer_compatibility_idx')
            UNION ALL
            SELECT 'tokenizer_search_tokenizer_idx', name, tokenizer
            FROM paradedb.schema('tokenizer_search_tokenizer_idx')
        ) current
        FULL JOIN tokenizer_compatibility_schema_before_upgrade old USING (index_name, name)
        WHERE current.index_name IS NULL
            OR old.index_name IS NULL
            OR current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'the upgrade must preserve all stored tokenizer names without REINDEX';
END;
$$;

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO tokenizer_compatibility VALUES (
    3,
    'Hello There 🚀',
    'Hello There 🚀',
    'hello there',
    'hello there',
    'hello there',
    'hello there',
    'hello there',
    'hello there',
    'compat',
    'hello there'
);

DO $$
BEGIN
    ASSERT NOT EXISTS (
        SELECT 1
        FROM tokenizer_compatibility_results current
        FULL JOIN tokenizer_compatibility_results_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM old.ids || ARRAY[2, 3]
    ), 'every tokenizer must keep matching old rows and new immutable rows';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_compatibility WHERE bio ||| 'hello'
    ), 'lowercase=false must keep lowercase input distinct after immutable writes';
    ASSERT NOT EXISTS (
        SELECT 1
        FROM (
            SELECT 'tokenizer_compatibility_idx' AS index_name, name, tokenizer
            FROM paradedb.schema('tokenizer_compatibility_idx')
            UNION ALL
            SELECT 'tokenizer_search_tokenizer_idx', name, tokenizer
            FROM paradedb.schema('tokenizer_search_tokenizer_idx')
        ) current
        FULL JOIN tokenizer_compatibility_schema_before_upgrade old USING (index_name, name)
        WHERE current.index_name IS NULL
            OR old.index_name IS NULL
            OR current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'immutable writes must not change the stored tokenizer names';
END;
$$;

RESET paradedb.global_mutable_segment_rows;

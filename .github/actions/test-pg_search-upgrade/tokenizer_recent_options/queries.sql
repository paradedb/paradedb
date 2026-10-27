SET plpgsql.check_asserts = on;
SET paradedb.global_mutable_segment_rows = 1000;
INSERT INTO tokenizer_recent_options (id) VALUES (2);

DO $$
BEGIN
    ASSERT EXISTS (SELECT 1 FROM paradedb.index_info('tokenizer_recent_options_idx') WHERE mutable),
        'the upgraded index must have a mutable segment';
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_recent_results current
        FULL JOIN tokenizer_recent_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM
            CASE WHEN old.ids = ARRAY[1] THEN ARRAY[1, 2] ELSE old.ids END
    ), 'recent tokenizer options must preserve results after mutable writes';
    ASSERT NOT EXISTS (
        SELECT 1 FROM paradedb.schema('tokenizer_recent_options_idx') current
        FULL JOIN tokenizer_recent_schema_before_upgrade old USING (name)
        WHERE current.name IS NULL OR old.name IS NULL OR current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'recent tokenizer names must remain unchanged after upgrade';
END;
$$;

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO tokenizer_recent_options (id) VALUES (3);
DO $$
BEGIN
    ASSERT NOT EXISTS (
        SELECT 1 FROM tokenizer_recent_results current
        FULL JOIN tokenizer_recent_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM
            CASE WHEN old.ids = ARRAY[1] THEN ARRAY[1, 2, 3] ELSE old.ids END
    ), 'recent tokenizer options must preserve results after immutable writes';
    ASSERT NOT EXISTS (
        SELECT 1 FROM paradedb.schema('tokenizer_recent_options_idx') current
        FULL JOIN tokenizer_recent_schema_before_upgrade old USING (name)
        WHERE current.name IS NULL OR old.name IS NULL OR current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'recent tokenizer names must remain unchanged after immutable writes';
END;
$$;
RESET paradedb.global_mutable_segment_rows;

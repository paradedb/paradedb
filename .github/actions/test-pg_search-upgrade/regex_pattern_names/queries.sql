SET plpgsql.check_asserts = on;
SET paradedb.global_mutable_segment_rows = 1000;

INSERT INTO regex_pattern_compatibility VALUES (2, 'abc123', 'abc123');
DO $$
BEGIN
    ASSERT EXISTS (
        SELECT 1 FROM paradedb.index_info('regex_pattern_compatibility_idx') WHERE mutable
    ), 'the first post-upgrade insert must exercise a mutable segment';
    ASSERT NOT EXISTS (
        SELECT 1 FROM regex_pattern_results current
        FULL JOIN regex_pattern_results_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM
            CASE WHEN old.ids = ARRAY[1] THEN ARRAY[1, 2] ELSE ARRAY[]::integer[] END
    ), 'mutable writes must retain both pattern-specific analyzers';
    ASSERT NOT EXISTS (
        SELECT 1 FROM paradedb.schema('regex_pattern_compatibility_idx') current
        FULL JOIN regex_pattern_schema_before_upgrade old USING (name)
        WHERE current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'mutable writes must preserve the persisted pattern-specific tokenizer names';
END;
$$;

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO regex_pattern_compatibility VALUES (3, 'abc123', 'abc123');
DO $$
BEGIN
    ASSERT NOT EXISTS (
        SELECT 1 FROM regex_pattern_results current
        FULL JOIN regex_pattern_results_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM
            CASE WHEN old.ids = ARRAY[1] THEN ARRAY[1, 2, 3] ELSE ARRAY[]::integer[] END
    ), 'immutable writes must retain both pattern-specific analyzers';
    ASSERT NOT EXISTS (
        SELECT 1 FROM paradedb.schema('regex_pattern_compatibility_idx') current
        FULL JOIN regex_pattern_schema_before_upgrade old USING (name)
        WHERE current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'immutable writes must preserve the persisted pattern-specific tokenizer names';
END;
$$;

RESET paradedb.global_mutable_segment_rows;

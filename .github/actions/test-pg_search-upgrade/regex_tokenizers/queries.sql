SET plpgsql.check_asserts = on;
-- Exercise immutable segment writes to the existing, legacy-named index.
SET paradedb.global_mutable_segment_rows = 0;

DO $$
BEGIN
    ASSERT NOT EXISTS (
        SELECT 1 FROM regex_results current
        FULL JOIN regex_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM old.ids
    ), 'regex query results changed after upgrade';
    ASSERT NOT EXISTS (
        SELECT 1 FROM (
            SELECT name, tokenizer FROM paradedb.schema('regex_patterns_idx')
            WHERE name IN ('digits', 'letters')
        ) current
        FULL JOIN regex_schema_before_upgrade old USING (name)
        WHERE current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'upgrade must preserve the stored legacy tokenizer names';
END;
$$;

-- Duplicate row 1's text: every legacy query that matched row 1 must now
-- match the new row too, and a query that did not match must remain empty.
INSERT INTO regex_patterns VALUES (4, 'abc123', 'abc123');
DO $$
BEGIN
    ASSERT NOT EXISTS (
        SELECT 1 FROM regex_results current
        FULL JOIN regex_before_upgrade old USING (query)
        WHERE current.ids IS DISTINCT FROM
            CASE WHEN 1 = ANY(old.ids) THEN old.ids || ARRAY[4] ELSE old.ids END
    ), 'inserts after upgrade must use the same analyzers as the old index';
END;
$$;

REINDEX INDEX regex_patterns_idx;
DO $$
BEGIN
    ASSERT (SELECT count(*) FROM regex_results WHERE ids = ARRAY[1, 4]) = 4,
        'REINDEX must give both exact and analyzed queries their field-specific patterns';
    ASSERT (SELECT tokenizer FROM paradedb.schema('regex_patterns_idx') WHERE name = 'digits')
        = 'regex_pattern:"[0-9]+"[remove_long=64,lowercase=true]',
        'REINDEX must persist the digits pattern and filters';
    ASSERT (SELECT tokenizer FROM paradedb.schema('regex_patterns_idx') WHERE name = 'letters')
        = 'regex_pattern:"[a-z]+"[remove_long=64,lowercase=true]',
        'REINDEX must persist the letters pattern and filters';
END;
$$;

SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO text_key VALUES ('Another Case-ID', 'inserted');
INSERT INTO uuid_key VALUES ('550e8400-e29b-41d4-a716-446655440001', 'inserted');
INSERT INTO json_key VALUES ('{"key": "inserted"}', 'inserted');

DO $$
DECLARE
    plan jsonb;
BEGIN
    ASSERT (SELECT count(*) FROM text_key WHERE body @@@ 'inserted') = 1,
        'inserted row missing from legacy text key index';
    ASSERT (SELECT count(*) FROM uuid_key WHERE body @@@ 'inserted') = 1,
        'inserted row missing from legacy UUID key index';
    ASSERT (SELECT count(*) FROM json_key WHERE body @@@ 'inserted') = 1,
        'inserted row missing from legacy JSON key index';
    ASSERT (SELECT count(*) FROM text_key WHERE id ### 'Original Case-ID') = 1,
        'phrase search failed on the original legacy text key';
    ASSERT (SELECT count(*) FROM text_key WHERE id ### 'Another Case-ID') = 1,
        'phrase search failed on a legacy text key inserted after upgrade';
    ASSERT (SELECT count(*) FROM text_key WHERE id @@@ 'original') = 0,
        'legacy text keys must retain case-sensitive whole-value tokenization';
    ASSERT (SELECT count(*) FROM json_key WHERE id->>'key' ### 'original') = 1,
        'phrase search failed on the legacy JSON key';

    EXECUTE $query$EXPLAIN (FORMAT JSON, COSTS OFF)
        SELECT id FROM text_key WHERE body @@@ 'original' AND id = 'Original Case-ID'
    $query$ INTO plan;
    plan := (plan #>> '{0,Plan,Tantivy Query}')::jsonb;
    ASSERT plan @? '$.**.term ? (@.field == "id" && @.value == "Original Case-ID")',
        'legacy text key equality must be pushed into the index';
    ASSERT NOT plan @? '$.**.heap_filter',
        'legacy text key equality must not become a heap filter';
END;
$$;

CREATE TABLE new_text_key (id TEXT, body TEXT);
INSERT INTO new_text_key VALUES ('Original Case-ID', 'new');
CREATE INDEX new_text_key_idx ON new_text_key USING paradedb (id, body) WITH (key_field = 'id');

DO $$
BEGIN
    ASSERT (SELECT count(*) FROM new_text_key WHERE id @@@ 'original') = 1,
        'new indexes must use normal field defaults even with the ignored key_field option';
END;
$$;

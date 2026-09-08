SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO text_key VALUES ('Another Case-ID', 'inserted');
INSERT INTO uuid_key VALUES ('550e8400-e29b-41d4-a716-446655440001', 'inserted');
INSERT INTO json_key VALUES ('{"key": "inserted"}', 'inserted');

DO $$
BEGIN
    ASSERT (SELECT count(*) FROM text_key WHERE body @@@ 'inserted') = 1,
        'inserted row missing from legacy text key index';
    ASSERT (SELECT count(*) FROM uuid_key WHERE body @@@ 'inserted') = 1,
        'inserted row missing from legacy UUID key index';
    ASSERT (SELECT count(*) FROM json_key WHERE body @@@ 'inserted') = 1,
        'inserted row missing from legacy JSON key index';
END;
$$;

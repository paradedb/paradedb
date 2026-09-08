SET paradedb.global_mutable_segment_rows = 0;

DO $$
BEGIN
    IF (SELECT count(*) FROM text_key WHERE id ||| 'Mixed Case-ID') <> 1 THEN
        RAISE EXCEPTION 'legacy text key tokenizer lost its literal behavior';
    END IF;
    IF (SELECT count(*) FROM text_key WHERE id ||| 'mixed case-id') <> 0 THEN
        RAISE EXCEPTION 'legacy text key tokenizer lost its case sensitivity';
    END IF;
    IF (SELECT count(*) FROM text_key WHERE id ### 'Mixed Case-ID') <> 1 THEN
        RAISE EXCEPTION 'legacy text key no longer supports phrase queries';
    END IF;
    IF (SELECT count(*) FROM uuid_key WHERE id @@@ '550e8400-e29b-41d4-a716-446655440000') <> 1 THEN
        RAISE EXCEPTION 'legacy UUID key tokenizer was not registered';
    END IF;
END;
$$;

CREATE TABLE fresh_text_key (id TEXT, body TEXT);
INSERT INTO fresh_text_key VALUES ('Mixed Case-ID', 'example');
CREATE INDEX fresh_text_key_idx ON fresh_text_key USING paradedb (id, body)
WITH (key_field = 'id');

DO $$
BEGIN
    IF (SELECT count(*) FROM fresh_text_key WHERE id ||| 'mixed') <> 1 THEN
        RAISE EXCEPTION 'fresh text field unexpectedly uses literal tokenization';
    END IF;
END;
$$;

INSERT INTO text_key VALUES ('Another Case-ID', 'example');
INSERT INTO uuid_key VALUES ('550e8400-e29b-41d4-a716-446655440001', 'example');

DO $$
BEGIN
    IF (SELECT count(*) FROM text_key WHERE id ||| 'Another Case-ID') <> 1 THEN
        RAISE EXCEPTION 'inserted text key did not retain literal tokenization';
    END IF;
    IF (SELECT count(*) FROM uuid_key WHERE id @@@ '550e8400-e29b-41d4-a716-446655440001') <> 1 THEN
        RAISE EXCEPTION 'inserted UUID key did not retain literal tokenization';
    END IF;
END;
$$;

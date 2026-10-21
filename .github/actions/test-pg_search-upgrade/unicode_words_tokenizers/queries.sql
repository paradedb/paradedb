SET plpgsql.check_asserts = on;
SET paradedb.global_mutable_segment_rows = 1000;

INSERT INTO unicode_words_tokenizers VALUES (2, 'hello again 🎉', 'Hello Again 🎉');

DO $$
BEGIN
    ASSERT EXISTS (
        SELECT 1 FROM paradedb.index_info('unicode_words_tokenizers_idx') WHERE mutable
    ), 'the upgraded index must have a mutable segment';
    ASSERT (
        SELECT array_agg(id ORDER BY id) FROM unicode_words_tokenizers
        WHERE about_me ||| 'HELLO'
    ) = ARRAY[1, 2], 'about_me must be searchable across old and mutable rows';
    ASSERT (
        SELECT array_agg(id ORDER BY id) FROM unicode_words_tokenizers
        WHERE bio ||| 'Hello'
    ) = ARRAY[1, 2], 'bio must be searchable across old and mutable rows';
    ASSERT NOT EXISTS (
        SELECT 1 FROM unicode_words_tokenizers WHERE bio ||| 'hello'
    ), 'bio must keep lowercase=false at query time';
END;
$$;

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO unicode_words_tokenizers VALUES (3, 'hello there', 'Hello There');

DO $$
BEGIN
    ASSERT (
        SELECT array_agg(id ORDER BY id) FROM unicode_words_tokenizers
        WHERE about_me ||| 'HELLO'
    ) = ARRAY[1, 2, 3], 'immutable writes must work with the legacy unicode_words schema';
    ASSERT (
        SELECT array_agg(id ORDER BY id) FROM unicode_words_tokenizers
        WHERE bio ||| 'Hello'
    ) = ARRAY[1, 2, 3], 'immutable writes must work with the legacy unicode_words schema';
    ASSERT NOT EXISTS (
        SELECT 1 FROM paradedb.schema('unicode_words_tokenizers_idx') current
        FULL JOIN unicode_words_schema_before_upgrade old USING (name)
        WHERE current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'the existing unicode_words schema must remain unchanged without REINDEX';
END;
$$;

RESET paradedb.global_mutable_segment_rows;

SET plpgsql.check_asserts = on;
SET paradedb.global_mutable_segment_rows = 1000;

INSERT INTO range_tokenizers VALUES (2, '[200,300)', '[2026-02-01,2026-03-01)');

DO $$
BEGIN
    ASSERT EXISTS (
        SELECT 1 FROM paradedb.index_info('range_tokenizers_idx') WHERE mutable
    ), 'the upgraded index must have a mutable segment';
    ASSERT (
        SELECT array_agg(id ORDER BY id) FROM range_tokenizers WHERE id @@@ paradedb.all()
    ) = ARRAY[1, 2], 'search must materialize new rows in the legacy range index';
    ASSERT (
        SELECT array_agg(id ORDER BY id) FROM range_tokenizers
        WHERE amount_range @@@ pdb.range_term('[250,260)'::int8range, 'Intersects')
    ) = ARRAY[2], 'the new amount range must be searchable';
    ASSERT (
        SELECT array_agg(id ORDER BY id) FROM range_tokenizers
        WHERE date_range @@@ pdb.range_term('[2026-02-10,2026-02-11)'::daterange, 'Intersects')
    ) = ARRAY[2], 'the new date range must be searchable';
END;
$$;

SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO range_tokenizers VALUES (3, '[300,400)', '[2026-03-01,2026-04-01)');

DO $$
BEGIN
    ASSERT (
        SELECT array_agg(id ORDER BY id) FROM range_tokenizers WHERE id @@@ paradedb.all()
    ) = ARRAY[1, 2, 3], 'immutable writes must also work with the legacy range schema';
    ASSERT NOT EXISTS (
        SELECT 1 FROM paradedb.schema('range_tokenizers_idx') current
        FULL JOIN range_schema_before_upgrade old USING (name)
        WHERE current.tokenizer IS DISTINCT FROM old.tokenizer
    ), 'the existing range schema must remain unchanged without REINDEX';
END;
$$;

RESET paradedb.global_mutable_segment_rows;

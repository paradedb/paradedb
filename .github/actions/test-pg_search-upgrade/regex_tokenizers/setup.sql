-- Runs on the prior release, starting at v0.25.0: use its supported options.
-- Equal filters intentionally make these patterns collide under the legacy name.
CREATE TABLE regex_patterns (id INTEGER PRIMARY KEY, digits TEXT, letters TEXT);
INSERT INTO regex_patterns VALUES
    (1, 'abc123', 'abc123'),
    (2, 'def456', 'def456'),
    (3, 'xyz789', 'xyz789');
CREATE INDEX regex_patterns_idx ON regex_patterns USING paradedb (id, digits, letters)
WITH (
    key_field = 'id',
    text_fields = '{
        "digits": {"tokenizer": {"type": "regex", "pattern": "[0-9]+", "lowercase": true, "remove_long": 64}},
        "letters": {"tokenizer": {"type": "regex", "pattern": "[a-z]+", "lowercase": true, "remove_long": 64}}
    }'
);

-- Keep both exact-term and analyzed-query results, rather than assuming which
-- pattern wins the old registration collision. This view is evaluated again
-- against the upgraded extension, including rows written after the upgrade.
CREATE VIEW regex_results AS
SELECT 'digits_term' AS query, ARRAY(
    SELECT id FROM regex_patterns WHERE id @@@ paradedb.term('digits', '123') ORDER BY id
) AS ids
UNION ALL
SELECT 'letters_term', ARRAY(
    SELECT id FROM regex_patterns WHERE id @@@ paradedb.term('letters', 'abc') ORDER BY id
)
UNION ALL
SELECT 'digits_parse', ARRAY(
    SELECT id FROM regex_patterns WHERE id @@@ 'digits:abc123' ORDER BY id
)
UNION ALL
SELECT 'letters_parse', ARRAY(
    SELECT id FROM regex_patterns WHERE id @@@ 'letters:abc123' ORDER BY id
);
CREATE TABLE regex_before_upgrade AS SELECT * FROM regex_results;
CREATE TABLE regex_schema_before_upgrade AS
SELECT name, tokenizer FROM paradedb.schema('regex_patterns_idx')
WHERE name IN ('digits', 'letters');

SET plpgsql.check_asserts = on;
DO $$
BEGIN
    ASSERT (SELECT count(*) FROM regex_before_upgrade WHERE query LIKE '%_parse' AND ids = ARRAY[1]) = 2,
        'legacy regex query tokenization must match the original row';
    ASSERT (SELECT count(*) FROM regex_before_upgrade WHERE query LIKE '%_term' AND ids = ARRAY[1]) = 1,
        'fixture must reproduce the legacy regex pattern collision';
    ASSERT (SELECT count(*) FROM regex_schema_before_upgrade WHERE tokenizer LIKE 'regex[%') = 2,
        'fixture must store legacy regex names with explicit filters';
END;
$$;

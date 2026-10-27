CREATE TABLE regex_pattern_compatibility (
    id INTEGER PRIMARY KEY,
    digits TEXT,
    letters TEXT
);
INSERT INTO regex_pattern_compatibility VALUES (1, 'abc123', 'abc123');

CREATE INDEX regex_pattern_compatibility_idx ON regex_pattern_compatibility
USING paradedb (
    id,
    (digits::pdb.regex_pattern('[0-9]+')),
    (letters::pdb.regex_pattern('[a-z]+'))
);

CREATE VIEW regex_pattern_results AS
SELECT 'digits' AS query, ARRAY(
    SELECT id FROM regex_pattern_compatibility
    WHERE digits ||| '123' ORDER BY id
) AS ids
UNION ALL
SELECT 'letters', ARRAY(
    SELECT id FROM regex_pattern_compatibility
    WHERE letters ||| 'abc' ORDER BY id
)
UNION ALL
SELECT 'digits_rejects_letters', ARRAY(
    SELECT id FROM regex_pattern_compatibility
    WHERE digits ||| 'abc' ORDER BY id
)
UNION ALL
SELECT 'letters_rejects_digits', ARRAY(
    SELECT id FROM regex_pattern_compatibility
    WHERE letters ||| '123' ORDER BY id
);

CREATE TABLE regex_pattern_results_before_upgrade AS SELECT * FROM regex_pattern_results;
CREATE TABLE regex_pattern_schema_before_upgrade AS
SELECT name, tokenizer FROM paradedb.schema('regex_pattern_compatibility_idx');

SET plpgsql.check_asserts = on;
DO $$
BEGIN
    ASSERT (SELECT count(*) FROM regex_pattern_results_before_upgrade WHERE ids = ARRAY[1]) = 2,
        'each regex pattern must match through its own field tokenizer before upgrade';
    ASSERT (SELECT count(*) FROM regex_pattern_results_before_upgrade WHERE ids = ARRAY[]::integer[]) = 2,
        'each regex pattern must reject terms handled only by the other pattern';
    ASSERT (SELECT count(*) FROM regex_pattern_schema_before_upgrade WHERE tokenizer LIKE 'regex_pattern:%') = 2,
        'the index must persist both pattern-specific tokenizer names';
END;
$$;

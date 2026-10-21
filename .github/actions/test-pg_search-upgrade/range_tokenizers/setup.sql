CREATE TABLE range_tokenizers (
    id INTEGER PRIMARY KEY,
    amount_range INT8RANGE,
    date_range DATERANGE
);

INSERT INTO range_tokenizers VALUES (1, '[100,200)', '[2026-01-01,2026-02-01)');

-- A text or JSON field could mask missing range tokenizer registration.
CREATE INDEX range_tokenizers_idx ON range_tokenizers
USING paradedb (id, amount_range, date_range) WITH (key_field = 'id');

CREATE TABLE range_schema_before_upgrade AS
SELECT name, tokenizer FROM paradedb.schema('range_tokenizers_idx');

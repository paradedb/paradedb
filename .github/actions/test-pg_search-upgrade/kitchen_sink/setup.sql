CALL paradedb.create_bm25_test_table(schema_name => 'public', table_name => 'mock_items');

ALTER TABLE mock_items
ADD COLUMN price NUMERIC(10, 2),
ADD COLUMN precise_price NUMERIC,
ADD COLUMN score DOUBLE PRECISION,
ADD COLUMN external_id UUID,
ADD COLUMN ip INET,
ADD COLUMN tags TEXT[],
ADD COLUMN active_period DATERANGE;

UPDATE mock_items
SET
    price = rating * 19.99,
    precise_price = 0.9554861284073128,
    score = rating::DOUBLE PRECISION / 3,
    external_id = ('00000000-0000-0000-0000-' || lpad(id::TEXT, 12, '0'))::UUID,
    ip = ('10.0.0.' || ((id % 250) + 1))::INET,
    tags = ARRAY[lower(category), split_part(lower(description), ' ', 1)],
    active_period = daterange(last_updated_date, last_updated_date + rating, '[]');

CREATE INDEX search_idx ON mock_items
USING bm25 (
    id,
    description,
    (category::pdb.literal),
    rating,
    in_stock,
    created_at,
    last_updated_date,
    latest_available_time,
    metadata,
    weight_range,
    price,
    precise_price,
    score,
    external_id,
    ip,
    tags,
    active_period,
    ((description || ' ' || category)::pdb.simple('alias=description_category')),
    (description::pdb.whitespace('alias=description_whitespace')),
    (description::pdb.source_code('alias=description_source_code')),
    (description::pdb.literal_normalized('alias=description_literal_normalized')),
    ((metadata->>'color')::pdb.ngram(2,3, 'alias=metadata_color'))
)
WITH (key_field='id');

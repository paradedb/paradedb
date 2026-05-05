DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pdb.verify_index('search_idx') WHERE NOT passed) THEN
        RAISE EXCEPTION 'search_idx failed verification';
    END IF;
END;
$$;

SELECT id, description, pdb.score(id), pdb.snippet(description)
FROM mock_items
WHERE description ||| 'running shoes'
ORDER BY pdb.score(id) DESC
LIMIT 5;

SELECT id, description
FROM mock_items
WHERE description::pdb.alias('description_whitespace') ||| 'running shoes'
ORDER BY id;

SELECT id, description
FROM mock_items
WHERE description::pdb.alias('description_source_code') ||| 'HTTPRequest source_code'
ORDER BY id
LIMIT 5;

SELECT id, description
FROM mock_items
WHERE description::pdb.alias('description_literal_normalized') === 'generic shoes'
ORDER BY id;

SELECT id, description, category
FROM mock_items
WHERE (description || ' ' || category) &&& 'running footwear'
ORDER BY id;

SELECT id, metadata->>'color'
FROM mock_items
WHERE metadata->>'color' @@@ 'whi'
ORDER BY id
LIMIT 5;

SELECT id, category
FROM mock_items
WHERE category === 'Footwear'
ORDER BY id;

SELECT id, weight_range
FROM mock_items
WHERE weight_range @@@ pdb.range_term('(2, 11]'::int4range, 'Intersects')
ORDER BY id
LIMIT 5;

SELECT id, active_period
FROM mock_items
WHERE active_period @@@ pdb.range_term('[2023-05-01,2023-05-15]'::daterange, 'Intersects')
ORDER BY id;

SELECT id, price, precise_price, score
FROM mock_items
WHERE id @@@ pdb.all() AND price > 50 AND precise_price > 100000000 AND score > 1
ORDER BY price
LIMIT 5;

SELECT id, external_id, ip
FROM mock_items
WHERE id @@@ pdb.all()
AND external_id IN ('00000000-0000-0000-0000-000000000003'::uuid, '00000000-0000-0000-0000-000000100001'::uuid)
ORDER BY id
LIMIT 5;

SELECT id, tags
FROM mock_items
WHERE tags === 'footwear'
ORDER BY id
LIMIT 5;

SELECT pdb.agg('{"value_count": {"field": "id"}}')
FROM mock_items
WHERE id @@@ pdb.all();

-- Insert some data

INSERT INTO mock_items (
    description,
    rating,
    category,
    in_stock,
    metadata,
    created_at,
    last_updated_date,
    latest_available_time,
    weight_range,
    price,
    precise_price,
    score,
    external_id,
    ip,
    tags,
    active_period
)
VALUES
    (
        'Waterproof running shoes',
        5,
        'Footwear',
        true,
        '{"color": "White", "location": "United States"}'::jsonb,
        '2023-05-10 10:00:00'::timestamp,
        '2023-05-10'::date,
        '10:00:00'::time,
        '[4,12]'::int4range,
        129.99,
        12345678901234567890.12345,
        9.5,
        '00000000-0000-0000-0000-000000100001'::uuid,
        '10.1.0.1'::inet,
        ARRAY['footwear', 'waterproof', 'running'],
        '[2023-05-10,2023-06-01]'::daterange
    ),
    (
        '',
        NULL,
        '',
        NULL,
        '{}'::jsonb,
        NULL,
        NULL,
        NULL,
        'empty'::int4range,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        ARRAY[]::TEXT[],
        'empty'::daterange
    );


-- Run the queries again after doing the inserts
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pdb.verify_index('search_idx') WHERE NOT passed) THEN
        RAISE EXCEPTION 'search_idx failed verification';
    END IF;
END;
$$;

SELECT id, description, pdb.score(id), pdb.snippet(description)
FROM mock_items
WHERE description ||| 'running shoes'
ORDER BY pdb.score(id) DESC
LIMIT 5;

SELECT id, description
FROM mock_items
WHERE description::pdb.alias('description_whitespace') ||| 'running shoes'
ORDER BY id;

SELECT id, description
FROM mock_items
WHERE description::pdb.alias('description_source_code') ||| 'HTTPRequest source_code'
ORDER BY id
LIMIT 5;

SELECT id, description
FROM mock_items
WHERE description::pdb.alias('description_literal_normalized') === 'generic shoes'
ORDER BY id;

SELECT id, description, category
FROM mock_items
WHERE (description || ' ' || category) &&& 'running footwear'
ORDER BY id;

SELECT id, metadata->>'color'
FROM mock_items
WHERE metadata->>'color' @@@ 'whi'
ORDER BY id
LIMIT 5;

SELECT id, category
FROM mock_items
WHERE category === 'Footwear'
ORDER BY id;

SELECT id, weight_range
FROM mock_items
WHERE weight_range @@@ pdb.range_term('(2, 11]'::int4range, 'Intersects')
ORDER BY id
LIMIT 5;

SELECT id, active_period
FROM mock_items
WHERE active_period @@@ pdb.range_term('[2023-05-01,2023-05-15]'::daterange, 'Intersects')
ORDER BY id;

SELECT id, price, precise_price, score
FROM mock_items
WHERE id @@@ pdb.all() AND price > 50 AND precise_price > 100000000 AND score > 1
ORDER BY price
LIMIT 5;

SELECT id, external_id, ip
FROM mock_items
WHERE id @@@ pdb.all()
AND external_id IN ('00000000-0000-0000-0000-000000000003'::uuid, '00000000-0000-0000-0000-000000100001'::uuid)
ORDER BY id
LIMIT 5;

SELECT id, tags
FROM mock_items
WHERE tags === 'footwear'
ORDER BY id
LIMIT 5;

SELECT pdb.agg('{"value_count": {"field": "id"}}')
FROM mock_items
WHERE id @@@ pdb.all();


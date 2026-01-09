\i common/common_setup.sql

-- jsonb_values field-agnostic search (const + runtime RHS)
DROP TABLE IF EXISTS jsonb_values_items;
CREATE TABLE jsonb_values_items (
  id SERIAL PRIMARY KEY,
  name TEXT,
  metadata JSONB,
  metadata_json JSON
);

INSERT INTO jsonb_values_items (name, metadata, metadata_json) VALUES
('air max', '{"color":"black","location":"usa","brand":{"name":"nike","line":"air max","country":"usa"},"details":{"release":{"season":"summer"}},"tags":["running","sale"]}', '{"color":"black"}'),
('ultra boost', '{"color":"white","location":"canada","brand":{"name":"adidas","line":"ultra boost","country":"germany"},"details":{"release":{"season":"winter"}},"tags":["running","winter"]}', '{"color":"white"}'),
('free run', '{"color":"green","location":"black","brand":{"name":"nike","line":"free run","country":"usa"},"details":{"release":{"season":"spring"}},"tags":["trail"]}', '{"color":"green"}'),
('speed cat', '{"color":"black","location":"uk","brand":{"name":"puma","line":"speed cat","country":"germany"},"details":{"release":{"season":"fall"}},"tags":["lifestyle"]}', '{"color":"black"}'),
('gel lyte', '{"color":"red","location":"usa","brand":{"name":"asics","line":"gel lyte","country":"japan"},"details":{"release":{"season":"summer"}},"tags":["retro","sale"]}', '{"color":"red"}');

CREATE INDEX jsonb_values_idx ON jsonb_values_items
USING bm25 (id, name, metadata, metadata_json)
WITH (key_field='id', jsonb_values_paths='{"metadata":["color","color","location","brand.name","brand.line","brand.country","details.release.season"],"metadata_json":["color"]}');

-- duplicate path entries are deduplicated

-- === term (text + text[] + boost + fuzzy)
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'black'
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === ARRAY['white', 'green']
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'black'::pdb.boost(2)
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'blak'::pdb.fuzzy(1)
ORDER BY id;

-- @@@ parse (text + pdb.query)
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values @@@ 'air'
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values @@@ pdb.term('ultra')
ORDER BY id;

-- ### phrase (text + text[] + slop + boost)
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ### 'air max'
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ### ARRAY['ultra', 'boost']
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ### ARRAY['air', 'max']::pdb.slop(1)
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ### ARRAY['air', 'max']::pdb.boost(2)
ORDER BY id;

-- ||| match disjunction (text + text[] + boost + fuzzy)
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ||| 'black'
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ||| ARRAY['nike', 'puma']
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ||| 'blak'::pdb.fuzzy(1)
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ||| 'black'::pdb.boost(2)
ORDER BY id;

-- &&& match conjunction (text + text[] + boost + fuzzy)
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values &&& 'black'
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values &&& ARRAY['air', 'max']
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values &&& 'black'::pdb.boost(2)
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values &&& 'blak'::pdb.fuzzy(1)
ORDER BY id;

-- sub-path scoping
SELECT id FROM jsonb_values_items
WHERE (metadata->'brand')::pdb.jsonb_values === 'nike'
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE (metadata->'brand')::pdb.jsonb_values === 'germany'
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE (metadata->'brand'->'name')::pdb.jsonb_values === 'nike'
ORDER BY id;

SELECT id FROM jsonb_values_items
WHERE (metadata->'details'->'release')::pdb.jsonb_values === 'summer'
ORDER BY id;

-- json column cast
SELECT id FROM jsonb_values_items
WHERE metadata_json::pdb.jsonb_values === 'black'
ORDER BY id;

-- jsonb_values_paths update (add tags)
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'sale'
ORDER BY id;

ALTER INDEX jsonb_values_idx SET (
  jsonb_values_paths='{"metadata":["color","location","brand.name","brand.line","brand.country","details.release.season","tags"],"metadata_json":["color"]}'
);

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'sale'
ORDER BY id;

-- runtime RHS expansion (prepared statements)
PREPARE jsonb_values_term(text) AS
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === $1
ORDER BY id;

EXECUTE jsonb_values_term('canada');
DEALLOCATE jsonb_values_term;

PREPARE jsonb_values_terms(text[]) AS
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === $1
ORDER BY id;

EXECUTE jsonb_values_terms(ARRAY['red', 'green']);
DEALLOCATE jsonb_values_terms;

PREPARE jsonb_values_parse(text) AS
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values @@@ $1
ORDER BY id;

EXECUTE jsonb_values_parse('boost');
DEALLOCATE jsonb_values_parse;

PREPARE jsonb_values_match(text) AS
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ||| $1
ORDER BY id;

EXECUTE jsonb_values_match('puma');
DEALLOCATE jsonb_values_match;

PREPARE jsonb_values_phrase(text) AS
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values ### $1
ORDER BY id;

EXECUTE jsonb_values_phrase('gel lyte');
DEALLOCATE jsonb_values_phrase;

PREPARE jsonb_values_conj(text) AS
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values &&& $1
ORDER BY id;

EXECUTE jsonb_values_conj('adidas');
DEALLOCATE jsonb_values_conj;

-- error: non-JSON field
SELECT id FROM jsonb_values_items
WHERE name::jsonb::pdb.jsonb_values === 'nike';

-- proximity clause with jsonb_values (no matches in test data)
SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values @@@ pdb.prox_clause(
  pdb.prox_term('running'), 5, pdb.prox_term('shoes')
);

DROP INDEX jsonb_values_idx;

-- error: field not in BM25 index
CREATE INDEX jsonb_values_no_field_idx ON jsonb_values_items
USING bm25 (id)
WITH (key_field='id');

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'black';

DROP INDEX IF EXISTS jsonb_values_no_field_idx;

-- error: jsonb_values_paths missing
CREATE INDEX jsonb_values_missing_idx ON jsonb_values_items
USING bm25 (id, metadata)
WITH (key_field='id');

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'black';

DROP INDEX IF EXISTS jsonb_values_missing_idx;

-- error: field missing from jsonb_values_paths
CREATE INDEX jsonb_values_missing_field_idx ON jsonb_values_items
USING bm25 (id, metadata, metadata_json)
WITH (key_field='id', jsonb_values_paths='{"metadata_json":["color"]}');

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'black';

DROP INDEX IF EXISTS jsonb_values_missing_field_idx;

-- error: no searchable paths under sub-path
CREATE INDEX jsonb_values_no_subpath_idx ON jsonb_values_items
USING bm25 (id, metadata)
WITH (key_field='id', jsonb_values_paths='{"metadata":["color"]}');

SELECT id FROM jsonb_values_items
WHERE (metadata->'brand')::pdb.jsonb_values === 'nike';

DROP INDEX IF EXISTS jsonb_values_no_subpath_idx;

-- error: jsonb_values_paths empty
CREATE INDEX jsonb_values_empty_idx ON jsonb_values_items
USING bm25 (id, metadata)
WITH (key_field='id', jsonb_values_paths='{"metadata": []}');

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'black';

DROP INDEX IF EXISTS jsonb_values_empty_idx;

-- error: expand_dots disabled
CREATE INDEX jsonb_values_nodots_idx ON jsonb_values_items
USING bm25 (id, metadata)
WITH (
  key_field='id',
  json_fields='{"metadata": {"expand_dots": false}}',
  jsonb_values_paths='{"metadata": ["color"]}'
);

SELECT id FROM jsonb_values_items
WHERE metadata::pdb.jsonb_values === 'black';

DROP INDEX IF EXISTS jsonb_values_nodots_idx;

-- error: invalid jsonb_values_paths
CREATE INDEX jsonb_values_bad_idx ON jsonb_values_items
USING bm25 (id, metadata)
WITH (key_field='id', jsonb_values_paths='{"metadata": ["color", "brand..name"]}');

DROP INDEX IF EXISTS jsonb_values_bad_idx;
DROP TABLE jsonb_values_items;

\i common/common_cleanup.sql

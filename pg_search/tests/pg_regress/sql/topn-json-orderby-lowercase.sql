\i common/common_setup.sql

DROP TABLE IF EXISTS mock_items_jsonlower;
CREATE TABLE mock_items_jsonlower (
    id SERIAL PRIMARY KEY,
    description TEXT,
    metadata JSONB
);

-- Mixed-case values so a case-folded (lowercase) sort disagrees with PG's
-- case-sensitive ->>. In ASCII, uppercase sorts before lowercase, so the correct
-- PG order ASC is: 'Black' < 'White' < 'black' < 'white'.
INSERT INTO mock_items_jsonlower (description, metadata) VALUES
('a', '{"color": "white"}'),
('b', '{"color": "Black"}'),
('c', '{"color": "black"}'),
('d', '{"color": "White"}');

-- JSON fast field configured with a LOWERCASE normalizer.
CREATE INDEX jsonlower_idx ON mock_items_jsonlower
USING bm25 (id, description, metadata)
WITH (key_field='id',
      json_fields='{"metadata": {"fast": true, "normalizer": "lowercase"}}');

-- A lowercase-normalized JSON fast field must NOT push the raw `metadata->>'color'`
-- sort down to Tantivy (the index stores case-folded values, which disagree with PG's
-- case-sensitive ->>). The planner should reject the pushdown and fall back to a PG
-- Sort node, so the EXPLAIN shows a `Sort` (not `TopK Order By: metadata.color ...`).
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color' AS color FROM mock_items_jsonlower
WHERE id @@@ paradedb.all()
ORDER BY color ASC, id ASC
LIMIT 5;

-- Results must follow PG's case-sensitive order: Black, White, black, white.
SELECT description, metadata->>'color' AS color FROM mock_items_jsonlower
WHERE id @@@ paradedb.all()
ORDER BY color ASC, id ASC
LIMIT 5;

DROP TABLE mock_items_jsonlower;

\i common/common_cleanup.sql

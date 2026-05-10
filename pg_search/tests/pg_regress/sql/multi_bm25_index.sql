-- A relation may only have a single `USING bm25` index, but the check is
-- bypassed when CREATE INDEX CONCURRENTLY is used (for the build-new/swap/drop-old
-- workflow). When two bm25 indexes coexist `rel_get_bm25_index` should pick the
-- highest-OID bm25 index, so a query referencing a field that only exists in the
-- newer index still resolves correctly.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS multi_bm25 CASCADE;
CREATE TABLE multi_bm25 (
    id SERIAL PRIMARY KEY,
    description TEXT,
    custom_identifiers JSONB NOT NULL DEFAULT '{}'::jsonb
);

INSERT INTO multi_bm25 (description, custom_identifiers) VALUES
    ('alpha', '{"invoice_number": "abc-001"}'),
    ('beta',  '{"invoice_number": "def-002"}');

-- Older index lacks `custom_identifiers` -- represents a previous schema.
CREATE INDEX CONCURRENTLY multi_bm25_old ON multi_bm25
USING bm25 (id, description) WITH (key_field = 'id');

-- Newer index adds `custom_identifiers`. CONCURRENTLY bypasses the
-- single-bm25-index restriction.
CREATE INDEX CONCURRENTLY multi_bm25_new ON multi_bm25
USING bm25 (id, description, (custom_identifiers::pdb.literal_normalized))
WITH (key_field = 'id');

-- A query against the field that only `multi_bm25_new` knows about should
-- succeed because we pick the highest-OID bm25 index, which is `multi_bm25_new`.
-- Without the fix this would error with `field 'custom_identifiers.invoice_number'
-- is not part of the pg_search index`.
SELECT id FROM multi_bm25
 WHERE custom_identifiers->>'invoice_number' &&& 'abc-001'
 ORDER BY id;

-- Drop the older index. From here only `multi_bm25_new` remains and the query
-- continues to work.
DROP INDEX multi_bm25_old;

SELECT id FROM multi_bm25
 WHERE custom_identifiers->>'invoice_number' &&& 'abc-001'
 ORDER BY id;

DROP TABLE multi_bm25 CASCADE;

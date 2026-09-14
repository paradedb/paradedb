\i common/common_setup.sql

CREATE TABLE partial_index_fallback (
    id int PRIMARY KEY,
    anchor text,
    body text NOT NULL,
    active boolean,
    priority int,
    note text NOT NULL DEFAULT 'before'
) WITH (fillfactor = 50);

INSERT INTO partial_index_fallback (id, anchor, body, active, priority) VALUES
    (1, NULL,        'allowed', true,  1),
    (2, 'duplicate', 'denied',  true,  1),
    (3, 'duplicate', 'allowed', false, 1),
    (4, NULL,        'allowed', NULL,  1),
    (5, NULL,        'allowed', true,  NULL),
    (6, 'duplicate', 'allowed', true,  0),
    (7, 'duplicate', 'denied',  false, 1);

-- Predicate columns need not be indexed; FALSE and NULL both require inline evaluation.
CREATE INDEX partial_index_fallback_idx ON partial_index_fallback
USING paradedb (anchor, id, body) WHERE active AND priority > 0;

SET paradedb.enable_custom_scan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;

SELECT id, body = 'allowed' AS native_match, body === 'allowed' AS search_match
FROM partial_index_fallback ORDER BY id;
SELECT id FROM partial_index_fallback WHERE body === 'allowed' ORDER BY id;
SELECT id FROM partial_index_fallback WHERE NOT (body === 'allowed') ORDER BY id;
SELECT id FROM partial_index_fallback WHERE id @@@ paradedb.all() ORDER BY id;

-- Citus sends deparsed filters to workers; both the marker and fallback must round-trip.
DO $$
DECLARE
    plan json;
    ids int[];
BEGIN
    EXECUTE 'EXPLAIN (FORMAT JSON) SELECT id FROM partial_index_fallback WHERE body === ''allowed'''
        INTO plan;
    ASSERT plan #>> '{0,Plan,Node Type}' = 'Seq Scan';
    EXECUTE format('SELECT ARRAY(SELECT id FROM partial_index_fallback WHERE %s ORDER BY id)',
                   plan #>> '{0,Plan,Filter}') INTO ids;
    ASSERT ids = ARRAY[1, 3, 4, 5, 6], 'reparsed filter must match indexed and unindexed rows';
END;
$$;

SET plan_cache_mode = force_generic_plan;
PREPARE partial_index_lookup(text) AS
SELECT id FROM partial_index_fallback WHERE body @@@ $1 ORDER BY id;
EXECUTE partial_index_lookup('allowed');
EXECUTE partial_index_lookup('denied');
DEALLOCATE partial_index_lookup;
RESET plan_cache_mode;

-- The predicate must be remapped independently for each table alias.
SELECT a.id, b.id
FROM partial_index_fallback a JOIN partial_index_fallback b ON a.id = b.id
WHERE a.body === 'allowed' AND b.body === 'allowed'
ORDER BY a.id;
SELECT a.id, b.id
FROM partial_index_fallback a LEFT JOIN partial_index_fallback b ON b.id = a.id + 1
WHERE b.id IS NULL OR b.body === 'allowed'
ORDER BY a.id;

-- Move matching rows into/out of the index, and change an unindexed row's search value.
UPDATE partial_index_fallback SET active = false WHERE id = 1;
UPDATE partial_index_fallback SET active = true WHERE id = 3;
UPDATE partial_index_fallback SET body = 'denied' WHERE id = 4;
SELECT id FROM partial_index_fallback WHERE body === 'allowed' ORDER BY id;

-- HOT updates and pruning must preserve both the indexed and inline paths.
UPDATE partial_index_fallback SET note = 'after' WHERE id IN (1, 3);
SELECT id, note FROM partial_index_fallback WHERE body === 'allowed' ORDER BY id;
VACUUM (FREEZE, ANALYZE) partial_index_fallback;
SELECT id, note FROM partial_index_fallback WHERE body === 'allowed' ORDER BY id;

-- An empty partial index is still usable as the schema for inline evaluation.
UPDATE partial_index_fallback SET active = false;
VACUUM (FREEZE, ANALYZE) partial_index_fallback;
SELECT id FROM partial_index_fallback WHERE body === 'allowed' ORDER BY id;

-- Inline documents must evaluate the same indexed expressions as stored documents.
DROP INDEX partial_index_fallback_idx;
UPDATE partial_index_fallback SET body = upper(body), active = (id % 2 = 1);
CREATE INDEX partial_index_fallback_idx ON partial_index_fallback
USING paradedb (anchor, id, (lower(body)::pdb.literal('alias=lower_body')))
WHERE active AND priority > 0;
SELECT id FROM partial_index_fallback WHERE lower(body) === 'allowed' ORDER BY id;

-- A strict helper must handle both the constant marker and real rows in a cached plan.
DROP INDEX partial_index_fallback_idx;
ALTER TABLE partial_index_fallback ADD COLUMN discarded text;
ALTER TABLE partial_index_fallback DROP COLUMN discarded;
CREATE INDEX partial_index_fallback_idx ON partial_index_fallback
USING paradedb (id, (lower(body)::pdb.literal('alias=lower_body')))
WHERE active AND priority > 0;
SET plan_cache_mode = force_generic_plan;
PREPARE strict_partial_lookup(text) AS
SELECT id, lower(body) @@@ $1 AS search_match FROM partial_index_fallback ORDER BY id;
EXECUTE strict_partial_lookup('allowed');
EXECUTE strict_partial_lookup('denied');
DEALLOCATE strict_partial_lookup;
RESET plan_cache_mode;

-- NULL-extended rows stay NULL, including under NOT; they are not inline documents.
SELECT n.id, lower(t.body) === 'allowed' AS search_match,
       NOT (lower(t.body) === 'allowed') AS negated_match
FROM generate_series(1, 8) AS n(id)
LEFT JOIN partial_index_fallback t ON t.id = n.id
ORDER BY n.id;

RESET enable_bitmapscan;
RESET enable_indexonlyscan;
RESET paradedb.enable_custom_scan;
DROP TABLE partial_index_fallback;

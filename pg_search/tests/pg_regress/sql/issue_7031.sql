-- Issue #7031: a NULL query argument that only becomes known at execution time
-- (from an InitPlan) must match nothing instead of crashing the backend on the
-- ParadeDB index AM path (amrescan received a ScanKey with SK_ISNULL set).
CREATE TABLE issue_7031_items (id integer PRIMARY KEY, state text NOT NULL);
INSERT INTO issue_7031_items SELECT g, 'active' FROM generate_series(1, 10000) g;
CREATE INDEX issue_7031_idx ON issue_7031_items USING paradedb (id, (state::pdb.literal));
ANALYZE issue_7031_items;
SET paradedb.planner_warnings = off;

-- The reported query: with default settings the planner picks a ParadeDB Index Scan.
SELECT explain_redact_oid($$
SELECT i.id FROM issue_7031_items i
WHERE i.state === 'active'
  AND i.id @@@ pdb.term_set((SELECT array_agg(id) FROM issue_7031_items WHERE id < 0))
$$);
SELECT i.id FROM issue_7031_items i
WHERE i.state === 'active'
  AND i.id @@@ pdb.term_set((SELECT array_agg(id) FROM issue_7031_items WHERE id < 0));

-- A non-NULL subquery result still returns rows.
SELECT i.id FROM issue_7031_items i
WHERE i.state === 'active'
  AND i.id @@@ pdb.term_set((SELECT array_agg(id) FROM issue_7031_items WHERE id <= 3))
ORDER BY i.id;

-- Plain Index Scan, with the NULL as the only scan key.
SET paradedb.enable_custom_scan = off;
SET enable_seqscan = off;
SET enable_bitmapscan = off;
EXPLAIN (COSTS OFF) SELECT i.id FROM issue_7031_items i
WHERE i.id @@@ pdb.term_set((SELECT array_agg(id) FROM issue_7031_items WHERE id < 0));
SELECT i.id FROM issue_7031_items i
WHERE i.id @@@ pdb.term_set((SELECT array_agg(id) FROM issue_7031_items WHERE id < 0));
SELECT i.id FROM issue_7031_items i
WHERE i.id @@@ pdb.term((SELECT max(id) FROM issue_7031_items WHERE id < 0));
SELECT i.id FROM issue_7031_items i
WHERE i.id @@@ (SELECT NULL::pdb.query);

-- Bitmap Heap Scan goes through amgetbitmap instead of amgettuple.
RESET enable_bitmapscan;
SET enable_indexscan = off;
EXPLAIN (COSTS OFF) SELECT i.id FROM issue_7031_items i
WHERE i.id @@@ pdb.term_set((SELECT array_agg(id) FROM issue_7031_items WHERE id < 0));
SELECT i.id FROM issue_7031_items i
WHERE i.id @@@ pdb.term_set((SELECT array_agg(id) FROM issue_7031_items WHERE id < 0));
SELECT i.id FROM issue_7031_items i
WHERE i.state === 'active'
  AND i.id @@@ (SELECT NULL::pdb.query);

-- The custom scan already returned no rows; keep it covered.
RESET enable_indexscan;
RESET enable_seqscan;
RESET paradedb.enable_custom_scan;
SELECT i.id FROM issue_7031_items i
WHERE i.id @@@ pdb.term_set((SELECT array_agg(id) FROM issue_7031_items WHERE id < 0));

RESET paradedb.planner_warnings;
DROP TABLE issue_7031_items;

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan = ON;

CREATE TABLE issue_6649_internal_names (
    id SERIAL PRIMARY KEY,
    agg_0 INTEGER,
    ctid_0 INTEGER,
    _ctid_0 INTEGER,
    __pdb_k0 TEXT,
    __pdb_m0 INTEGER,
    __grouping_id INTEGER,
    category TEXT,
    amount NUMERIC(10, 2)
);

INSERT INTO issue_6649_internal_names
    (agg_0, ctid_0, _ctid_0, __pdb_k0, __pdb_m0, __grouping_id, category, amount)
SELECT
    g % 3,
    g % 3,
    g % 4,
    CASE WHEN g % 3 = 0 THEN 'alpha' WHEN g % 3 = 1 THEN 'beta' ELSE 'gamma' END,
    g % 2,
    g % 3,
    CASE WHEN g % 2 = 0 THEN 'even' ELSE 'odd' END,
    g * 10.25
FROM generate_series(1, 6) AS g;

CREATE INDEX issue_6649_internal_names_idx ON issue_6649_internal_names
USING paradedb (
    id,
    agg_0,
    ctid_0,
    _ctid_0,
    (__pdb_k0::pdb.literal),
    __pdb_m0,
    __grouping_id,
    (category::pdb.literal),
    amount
);

-- Keep the original aggregate-alias collision covered as well.
SELECT agg_0, SUM(amount)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY agg_0
ORDER BY agg_0;

-- A user column named like a synthetic CTID must remain distinct from it.
SELECT ctid_0, SUM(amount)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY ctid_0
ORDER BY ctid_0;

-- Synthetic CTID aliases must also avoid non-grouped aggregate input fields.
EXPLAIN (COSTS OFF, TIMING OFF, VERBOSE)
SELECT agg_0, SUM(_ctid_0), SUM(amount)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY agg_0
ORDER BY agg_0;

SELECT agg_0, SUM(_ctid_0), SUM(amount)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY agg_0
ORDER BY agg_0;

-- Join-key CTIDs use the same alias as the projected synthetic source fields.
-- EXPLAIN covers planning; execution currently exercises a separate pre-filter
-- limitation for dynamic filters on CTID columns.
EXPLAIN (COSTS OFF, TIMING OFF, VERBOSE)
SELECT a.agg_0, SUM(a.amount)
FROM issue_6649_internal_names AS a
JOIN issue_6649_internal_names AS b ON a.ctid = b.ctid
WHERE a.id @@@ paradedb.all()
GROUP BY a.agg_0
ORDER BY a.agg_0;

-- These group keys overlap with the synthetic columns produced by pdb.agg().
EXPLAIN (COSTS OFF, TIMING OFF, VERBOSE)
SELECT __pdb_k0, COUNT(*), SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}},"aggs":{"min_agg_0":{"min":{"field":"agg_0"}}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY __pdb_k0
ORDER BY __pdb_k0;

SELECT __pdb_k0, COUNT(*), SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}},"aggs":{"min_agg_0":{"min":{"field":"agg_0"}}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY __pdb_k0
ORDER BY __pdb_k0;

EXPLAIN (COSTS OFF, TIMING OFF, VERBOSE)
SELECT __pdb_m0, COUNT(*), SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}},"aggs":{"min_agg_0":{"min":{"field":"agg_0"}}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY __pdb_m0
ORDER BY __pdb_m0;

SELECT __pdb_m0, COUNT(*), SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}},"aggs":{"min_agg_0":{"min":{"field":"agg_0"}}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY __pdb_m0
ORDER BY __pdb_m0;

EXPLAIN (COSTS OFF, TIMING OFF, VERBOSE)
SELECT __grouping_id, COUNT(*), SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}},"aggs":{"min_agg_0":{"min":{"field":"agg_0"}}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY __grouping_id
ORDER BY __grouping_id;

SELECT __grouping_id, COUNT(*), SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}},"aggs":{"min_agg_0":{"min":{"field":"agg_0"}}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY __grouping_id
ORDER BY __grouping_id;

-- HAVING must read the renamed synthetic grouping ID on the scalar-key path.
EXPLAIN (COSTS OFF, TIMING OFF, VERBOSE)
SELECT agg_0, COUNT(*), SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}},"aggs":{"min_agg_0":{"min":{"field":"agg_0"}}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY agg_0
HAVING COUNT(*) > 1
ORDER BY agg_0;

SELECT agg_0, COUNT(*), SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}},"aggs":{"min_agg_0":{"min":{"field":"agg_0"}}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY agg_0
HAVING COUNT(*) > 1
ORDER BY agg_0;

DROP TABLE issue_6649_internal_names;

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan = ON;

CREATE TABLE issue_6649_internal_names (
    id SERIAL PRIMARY KEY,
    agg_0 INTEGER,
    ctid_0 INTEGER,
    __pdb_k0 TEXT,
    __pdb_m0 INTEGER,
    __grouping_id INTEGER,
    category TEXT,
    amount NUMERIC(10, 2)
);

INSERT INTO issue_6649_internal_names
    (agg_0, ctid_0, __pdb_k0, __pdb_m0, __grouping_id, category, amount)
SELECT
    g % 3,
    g % 3,
    CASE WHEN g % 3 = 0 THEN 'alpha' WHEN g % 3 = 1 THEN 'beta' ELSE 'gamma' END,
    g % 2,
    g % 3,
    CASE WHEN g % 2 = 0 THEN 'even' ELSE 'odd' END,
    g * 10.25
FROM generate_series(1, 6) AS g;

CREATE INDEX issue_6649_internal_names_idx ON issue_6649_internal_names
USING paradedb (id, agg_0, ctid_0, __pdb_k0, __pdb_m0, __grouping_id, category, amount);

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

-- These group keys overlap with the synthetic columns produced by pdb.agg().
SELECT __pdb_k0, SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY __pdb_k0
ORDER BY __pdb_k0;

SELECT __pdb_m0, SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY __pdb_m0
ORDER BY __pdb_m0;

SELECT __grouping_id, SUM(amount),
       pdb.agg('{"terms":{"size":10,"field":"category","order":{"_key":"asc"}}}'::jsonb)
FROM issue_6649_internal_names
WHERE id @@@ paradedb.all()
GROUP BY __grouping_id
ORDER BY __grouping_id;

DROP TABLE issue_6649_internal_names;

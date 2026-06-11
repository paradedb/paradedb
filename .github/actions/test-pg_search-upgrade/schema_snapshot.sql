-- Emits a stable, sorted, human-readable description of every object owned by
-- the pg_search extension (functions, types, casts, operators, views, etc.),
-- via the `e`-type dependencies that `CREATE EXTENSION` / `ALTER EXTENSION
-- UPDATE` record in pg_depend.
--
-- Run against a freshly-installed database and an upgraded database and diff the
-- two: any difference means a migration script under pg_search/sql/ is
-- incomplete -- an object lives in the generated base schema but is never
-- created on the `ALTER EXTENSION ... UPDATE` path (or vice versa).
--
-- pg_describe_object() renders OID-free, fully-qualified identifiers (e.g.
-- `function paradedb.verify_index(regclass,boolean,...)`), so the output is
-- comparable across databases with different OIDs.
SELECT pg_describe_object(d.classid, d.objid, d.objsubid) AS object
FROM pg_depend d
JOIN pg_extension e ON e.oid = d.refobjid
WHERE d.refclassid = 'pg_extension'::regclass
  AND d.deptype = 'e'
  AND e.extname = 'pg_search'
ORDER BY 1;

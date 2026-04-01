-- Shape: Scalar COUNT(*) on JOIN
-- Join: files → pages
-- Description: Count total joined rows matching a search predicate.
-- This is the simplest aggregate-on-join shape and exercises the
-- DataFusion backend's basic scan → join → aggregate pipeline.

-- Postgres default plan (custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*)
FROM files f
JOIN pages p ON f.id = p."fileId"
WHERE f.content @@@ 'Section';

-- DataFusion aggregate scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*)
FROM files f
JOIN pages p ON f.id = p."fileId"
WHERE f.content @@@ 'Section';

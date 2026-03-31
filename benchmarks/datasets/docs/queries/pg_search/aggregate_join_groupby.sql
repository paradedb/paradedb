-- Shape: GROUP BY aggregate on JOIN
-- Join: files → pages
-- Description: Grouped aggregate (COUNT, SUM) with GROUP BY on the parent
-- table's title column. Exercises the DataFusion backend's grouped
-- aggregate pipeline including custom_scan_tlist for scanrelid=0.

-- Postgres default plan (custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT f.title, COUNT(*), SUM(p."sizeInBytes")
FROM files f
JOIN pages p ON f.id = p."fileId"
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title;

-- DataFusion aggregate scan
SET paradedb.enable_aggregate_custom_scan TO on;
SELECT f.title, COUNT(*), SUM(p."sizeInBytes")
FROM files f
JOIN pages p ON f.id = p."fileId"
WHERE f.content @@@ 'Section'
GROUP BY f.title
ORDER BY f.title;

-- Shape: Multiple Aggregates on JOIN
-- Join: files → pages
-- Description: Multiple aggregate functions (COUNT, SUM, MIN, MAX) on a join.
-- Exercises the DataFusion backend's ability to compute multiple aggregates
-- in a single pass over the joined data.

-- Postgres default plan (custom scan off)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(p."sizeInBytes"), MAX(p."sizeInBytes")
FROM files f
JOIN pages p ON f.id = p."fileId"
WHERE f.content ||| 'Section';

-- DataFusion aggregate scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(p."sizeInBytes"), MAX(p."sizeInBytes")
FROM files f
JOIN pages p ON f.id = p."fileId"
WHERE f.content ||| 'Section';

-- MPP aggregate scan
SET statement_timeout TO '300s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT COUNT(*), MIN(p."sizeInBytes"), MAX(p."sizeInBytes")
FROM files f
JOIN pages p ON f.id = p."fileId"
WHERE f.content ||| 'Section';

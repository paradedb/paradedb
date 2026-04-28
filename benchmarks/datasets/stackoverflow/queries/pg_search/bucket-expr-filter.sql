SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year;

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year;

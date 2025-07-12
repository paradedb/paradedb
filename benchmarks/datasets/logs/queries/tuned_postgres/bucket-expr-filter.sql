SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') GROUP BY year ORDER BY year;

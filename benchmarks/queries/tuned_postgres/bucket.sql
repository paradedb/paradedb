SELECT country, COUNT(*) FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') GROUP BY country ORDER BY country;
SELECT severity, COUNT(*) FROM benchmark_logs WHERE to_tsvector('english', message) @@ to_tsquery('english', 'research') GROUP BY severity ORDER BY severity;

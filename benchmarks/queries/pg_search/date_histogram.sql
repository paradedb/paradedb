SELECT date_trunc('month', timestamp) as month, COUNT(*) FROM benchmark_logs WHERE message @@@ 'the' GROUP BY month ORDER BY month;
SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'the' GROUP BY year ORDER BY year;

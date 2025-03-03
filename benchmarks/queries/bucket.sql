SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'the' GROUP BY country ORDER BY country;
SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'the' GROUP BY severity ORDER BY severity;

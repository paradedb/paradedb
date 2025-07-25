-- numeric ff
SELECT metadata->>'value', COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY metadata->>'value' ORDER BY metadata->>'value';

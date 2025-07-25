SELECT * FROM benchmark_logs WHERE message @@@ 'research' and severity < 5 order by metadata->>'value' limit 5;

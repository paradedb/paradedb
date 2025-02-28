SELECT * FROM benchmark_logs WHERE message @@@ 'fox' AND severity @@@ '<3' LIMIT 10;
SELECT * FROM benchmark_logs WHERE message @@@ 'fox' AND timestamp @@@ '[2020-10-02T15:00:00Z TO *}' LIMIT 10;

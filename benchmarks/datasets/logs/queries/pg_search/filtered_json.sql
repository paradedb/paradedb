SELECT * FROM benchmark_logs WHERE id @@@ pdb.parse('metadata.label:"critical system alert"') AND message ||| 'research' AND severity < 3 LIMIT 10;

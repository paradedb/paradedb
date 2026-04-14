SELECT * FROM benchmark_logs WHERE id @@@ pdb.parse('metadata.label:"critical system alert"') AND id @@@ pdb.parse('metadata.value:[10 TO *]') AND message ||| 'research' LIMIT 10;

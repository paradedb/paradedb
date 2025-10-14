SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex_term('united.*') AND country ILIKE '% States');

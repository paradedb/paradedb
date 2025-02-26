| Query Type | Query | Mean (ms) | Std Dev (ms) | TPS | Runs | Rows Returned |
|------------|--------|-----------|--------------|-----|------|---------------|
| bucket | `SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'the' GROUP BY country ORDER BY country` | 26811.7 | 0 | 26.811736 | 1000 | 7 |
| bucket | `SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'the' GROUP BY severity ORDER BY severity` | 39676.1 | 0 | 39.676084 | 1000 | 5 |
| cardinality | `SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'the'` | 39039.3 | 0 | 39.039320 | 1000 | 1 |
| count | `SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()` | 49045.6 | 0 | 49.045573 | 1000 | 1 |
| count | `SELECT COUNT(*) FROM benchmark_logs WHERE country @@@ 'canada'` | 56749.2 | 0 | 56.749180 | 1000 | 1 |
| date_histogram | `SELECT date_trunc('month', timestamp) as month, COUNT(*) FROM benchmark_logs WHERE message @@@ 'the' GROUP BY month ORDER BY month` | 46892.7 | 0 | 46.892658 | 1000 | 24 |
| date_histogram | `SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'the' GROUP BY year ORDER BY year` | 47095.2 | 0 | 47.095170 | 1000 | 2 |
| filtering | `SELECT * FROM benchmark_logs WHERE message @@@ 'fox' AND severity @@@ '<3' LIMIT 10` | 380865 | 0 | 380.865326 | 1000 | 10 |
| filtering | `SELECT * FROM benchmark_logs WHERE message @@@ 'fox' AND timestamp @@@ '[2020-10-02T15:00:00Z TO *}' LIMIT 10` | 390046 | 0 | 390.046025 | 1000 | 10 |
| highlighting | `SELECT id, paradedb.snippet(message) FROM benchmark_logs WHERE message @@@ 'fox' LIMIT 10` | 254246 | 0 | 254.245907 | 1000 | 10 |
| json | `SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical') LIMIT 10` | 391926 | 0 | 391.926318 | 1000 | 10 |
| json | `SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) LIMIT 10` | 355961 | 0 | 355.960560 | 1000 | 10 |
| phrase | `SELECT * FROM benchmark_logs WHERE message @@@ '"quick brown fox"' OR country @@@ '"United States"' LIMIT 10` | 195794 | 0 | 195.794338 | 1000 | 10 |
| term | `SELECT * FROM benchmark_logs WHERE message @@@ 'fox' OR country @@@ 'canada' LIMIT 10` | 361428 | 0 | 361.428365 | 1000 | 10 |
| term | `SELECT * FROM benchmark_logs WHERE message @@@ 'fox' OR country @@@ 'canada' LIMIT 250` | 291452 | 0 | 291.451721 | 1000 | 250 |
| top_n | `SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'fox' ORDER BY paradedb.score(id) LIMIT 10` | 67764 | 0 | 67.763992 | 1000 | 10 |
| top_n | `SELECT * FROM benchmark_logs WHERE message @@@ 'fox' ORDER BY severity LIMIT 10` | 66677.8 | 0 | 66.677780 | 1000 | 10 |
| top_n | `SELECT * FROM benchmark_logs WHERE message @@@ 'fox' ORDER BY country LIMIT 10` | 68729.9 | 0 | 68.729939 | 1000 | 10 |
| top_n | `SELECT * FROM benchmark_logs WHERE message @@@ 'fox' ORDER BY timestamp LIMIT 10` | 67582.2 | 0 | 67.582180 | 1000 | 10 |

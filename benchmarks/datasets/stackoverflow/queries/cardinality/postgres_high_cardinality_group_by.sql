-- high-cardinality aggregate scan
SET paradedb.enable_aggregate_custom_scan TO off; SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags LIMIT 65000;

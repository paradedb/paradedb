-- string ff
SET paradedb.enable_aggregate_custom_scan TO off; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name;


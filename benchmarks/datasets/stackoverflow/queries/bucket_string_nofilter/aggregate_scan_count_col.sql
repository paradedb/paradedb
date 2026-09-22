-- pdb.agg (under the hood) with GROUP BY
SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name;

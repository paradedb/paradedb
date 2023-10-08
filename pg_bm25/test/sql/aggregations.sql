-- Bucket aggregation
SELECT * FROM paradedb.aggregation('idxaggregations', '{"aggs": {"histogram": {"field": "rating", "interval": 2}}}');

-- Metrics aggregation
SELECT * FROM paradedb.aggregation('idxaggregations', '{"aggs": {"avg": {"field": "rating"}}}');

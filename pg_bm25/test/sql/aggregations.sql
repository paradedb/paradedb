-- Bucket aggregation 
SELECT * FROM paradedb.aggregation('aggregations_bm25_index', '{"aggs": {histogram: {field: "rating", interval: 2}}}');

-- Metrics aggregation
SELECT * FROM paradedb.aggregation('aggregations_bm25_index', '{"aggs": {avg: {field: "rating"}}}');

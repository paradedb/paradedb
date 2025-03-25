# Performance Tuning

This guide provides recommendations for optimizing search performance in pg_search, covering index configuration, query optimization, and relevant PostgreSQL settings.

## Index Configuration

Proper index configuration is crucial for optimal search performance. Consider the following tips:

1. **Field Types**: Choose appropriate field types for your data. For example:
   - Use `Text` for full-text search fields
   - Use `Numeric` for integer or floating-point values
   - Use `Date` for timestamp fields

2. **Indexing Options**: Configure indexing options based on your search requirements:
   - Set `indexed: true` for fields you need to search on
   - Use `fast: true` for fields that require sorting or aggregations
   - Set `stored: true` only for fields you need to retrieve in search results

3. **Tokenization**: Choose the right tokenizer for text fields:
   - Use `SearchTokenizer::default()` for general text
   - Use `SearchTokenizer::Raw` for exact match fields like UUIDs

Example configuration:

```rust
SearchFieldConfig::Text {
    indexed: true,
    fast: true,
    stored: false,
    fieldnorms: true,
    tokenizer: SearchTokenizer::default(),
    record: IndexRecordOption::WithFreqsAndPositions,
    normalizer: SearchNormalizer::Raw,
    column: None,
}
```

## Query Optimization

Optimize your queries for better performance:

1. **Field Selection**: Only request fields you need in the results to reduce I/O and memory usage.

2. **Query Structure**: Use structured queries when possible, as they can be more efficiently processed than free-text queries.

3. **Filtering**: Apply filters to reduce the number of documents that need to be scored.

4. **Sorting**: When sorting, use fields configured with `fast: true` for better performance.

5. **Limit Results**: Use reasonable limits on result sets to avoid unnecessary processing.

Example optimized query:

```sql
SELECT id, title, abstract
FROM documents
WHERE bm25(@@('title:performance AND abstract:optimization'))
ORDER BY publication_date DESC
LIMIT 100;
```

## PostgreSQL Settings

Adjust these PostgreSQL settings to potentially improve pg_search performance:

1. **shared_buffers**: Increase this value to allow more data to be cached in memory.

2. **work_mem**: A higher value can improve performance for complex sorts and hash operations.

3. **maintenance_work_mem**: Increase for faster index creation and vacuum operations.

4. **effective_cache_size**: Set this to an estimate of available memory for disk caching.

5. **max_parallel_workers_per_gather**: Increase to allow more parallel query execution.

Example configuration in `postgresql.conf`:

```
shared_buffers = 4GB
work_mem = 64MB
maintenance_work_mem = 256MB
effective_cache_size = 12GB
max_parallel_workers_per_gather = 4
```

Remember to test these settings in a staging environment before applying them to production.

## Monitoring and Profiling

Regular monitoring and profiling can help identify performance bottlenecks:

1. Use PostgreSQL's EXPLAIN ANALYZE to understand query execution plans.
2. Monitor system resources (CPU, memory, I/O) during search operations.
3. Use pg_stat_statements to track frequently executed and time-consuming queries.

By following these guidelines and continuously monitoring your system, you can optimize the performance of pg_search for your specific use case.